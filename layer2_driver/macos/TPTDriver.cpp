/*
 * TPTDriver.cpp — TPT GPU macOS DriverKit implementation
 *
 * DriverKit extensions run in userspace (dext), not the kernel.
 * PCI device access goes through PCIDriverKit APIs.
 */

#include "TPTDriver.h"
#include <DriverKit/DriverKit.h>
#include <PCIDriverKit/PCIDriverKit.h>

/* External method selector table */
enum TPTSelector {
    kTPTSelectorGemCreate  = 0,
    kTPTSelectorGemFree    = 1,
    kTPTSelectorGemInfo    = 2,
    kTPTSelectorSubmit     = 3,
    kTPTSelectorWaitFence  = 4,
    kTPTSelectorQueryInfo  = 5,
    kTPTSelectorCount,
};

// ==========================================================================
// TPTDriver::Start
// ==========================================================================
bool TPTDriver::Start(IOService *provider)
{
    if (!super::Start(provider)) return false;

    fPCIDevice = OSDynamicCast(IOPCIDevice, provider);
    if (!fPCIDevice) {
        Stop(provider);
        return false;
    }
    fPCIDevice->retain();

    /* Enable bus-master and memory-space access. */
    fPCIDevice->SetMemoryEnable(true);
    fPCIDevice->SetBusMasterEnable(true);

    /* Map BAR0 (register space). */
    kern_return_t kr = fPCIDevice->MapDeviceMemoryWithRegister(
        kIOPCIConfigBaseAddress0,
        kTPTBar0Index,
        &fBar0Map);
    if (kr != kIOReturnSuccess || !fBar0Map) {
        IOLog("TPT GPU: failed to map BAR0 (0x%x)\n", kr);
        Stop(provider);
        return false;
    }

    /* Map BAR2 (VRAM aperture). */
    kr = fPCIDevice->MapDeviceMemoryWithRegister(
        kIOPCIConfigBaseAddress2,
        kTPTBar2Index,
        &fBar2Map);
    if (kr != kIOReturnSuccess || !fBar2Map) {
        IOLog("TPT GPU: failed to map BAR2 (0x%x)\n", kr);
        Stop(provider);
        return false;
    }

    /* Initialise fence lock. */
    fFenceLock = IOLockAlloc();
    if (!fFenceLock) {
        Stop(provider);
        return false;
    }

    /* Configure interrupt dispatch source. */
    kr = IOInterruptDispatchSource::Create(
        fPCIDevice,
        0,   /* interrupt index 0 (MSI-X vector 0) */
        GetWorkQueue(),
        &fInterruptSource);
    if (kr != kIOReturnSuccess) {
        IOLog("TPT GPU: failed to create interrupt source (0x%x)\n", kr);
        Stop(provider);
        return false;
    }

    /* Create action block for interrupt callback. */
    kr = CreateActionInterruptOccurred(sizeof(uintptr_t), &fInterruptAction);
    if (kr != kIOReturnSuccess) {
        Stop(provider);
        return false;
    }
    fInterruptSource->SetHandler(fInterruptAction);
    fInterruptSource->SetEnable(true, nullptr);

    /* Enable interrupt mask and scheduler. */
    WriteReg32(kRegIrqMask, kIrqFenceSignaled | kIrqError);
    WriteReg32(kRegSchedEnable, 1);

    IOLog("TPT GPU: device started, VRAM size = %u MiB\n",
          ReadReg32(kRegVramSize) >> 20);

    RegisterService();
    return true;
}

// ==========================================================================
// TPTDriver::Stop
// ==========================================================================
void TPTDriver::Stop(IOService *provider)
{
    if (fInterruptSource) {
        fInterruptSource->SetEnable(false, nullptr);
        fInterruptSource->release();
        fInterruptSource = nullptr;
    }
    if (fInterruptAction) {
        fInterruptAction->release();
        fInterruptAction = nullptr;
    }
    if (fBar0Map) {
        /* Mask all interrupts before releasing BAR. */
        WriteReg32(kRegIrqMask, 0);
        WriteReg32(kRegSchedEnable, 0);
        fBar0Map->release();
        fBar0Map = nullptr;
    }
    if (fBar2Map) {
        fBar2Map->release();
        fBar2Map = nullptr;
    }
    if (fFenceLock) {
        IOLockFree(fFenceLock);
        fFenceLock = nullptr;
    }
    if (fPCIDevice) {
        fPCIDevice->release();
        fPCIDevice = nullptr;
    }
    super::Stop(provider);
}

void TPTDriver::free()
{
    super::free();
}

// ==========================================================================
// Interrupt handler (runs on work queue)
// ==========================================================================
void TPTDriver::HandleInterrupt()
{
    uint32_t status = ReadReg32(kRegIrqStatus);
    if (!status) return;

    WriteReg32(kRegIrqAck, status);

    if (status & kIrqError) {
        IOLog("TPT GPU: hardware error interrupt (status=0x%08x)\n", status);
        IOLockLock(fFenceLock);
        fHardwareError = true;
        IOLockUnlock(fFenceLock);
    }

    if (status & kIrqFenceSignaled) {
        uint32_t seqno = ReadReg32(kRegFenceSeqno);
        IOLockLock(fFenceLock);
        fCompletedSeqno = seqno;
        IOLockUnlock(fFenceLock);
        /* Wake all waiters via semaphore or dispatch group in a real driver. */
    }
}

// ==========================================================================
// Register accessors
// ==========================================================================
uint32_t TPTDriver::ReadReg32(uint32_t offset)
{
    auto *base = reinterpret_cast<volatile uint32_t *>(
        fBar0Map->GetAddress());
    return __builtin_bswap32(base[offset / 4]);  /* LE hardware; no-op on x86/arm64 */
}

void TPTDriver::WriteReg32(uint32_t offset, uint32_t value)
{
    auto *base = reinterpret_cast<volatile uint32_t *>(
        fBar0Map->GetAddress());
    base[offset / 4] = __builtin_bswap32(value);
    __sync_synchronize();
}

// ==========================================================================
// TPTUserClient
// ==========================================================================
bool TPTUserClient::Start(IOService *provider)
{
    if (!super::Start(provider)) return false;
    fDriver = OSDynamicCast(TPTDriver, provider);
    if (!fDriver) { Stop(provider); return false; }
    fDriver->retain();
    return true;
}

void TPTUserClient::Stop(IOService *provider)
{
    if (fDriver) { fDriver->release(); fDriver = nullptr; }
    super::Stop(provider);
}

void TPTUserClient::free() { super::free(); }

kern_return_t TPTUserClient::ExternalMethod(
    uint64_t                          selector,
    IOUserClientMethodArguments      *arguments,
    const IOUserClientMethodDispatch *dispatch,
    OSObject                         *target,
    void                             *reference)
{
    if (selector >= kTPTSelectorCount) return kIOReturnBadArgument;

    switch ((TPTSelector)selector) {
    case kTPTSelectorGemCreate:  return GemCreate(arguments);
    case kTPTSelectorGemFree:    return GemFree(arguments);
    case kTPTSelectorGemInfo:    return GemInfo(arguments);
    case kTPTSelectorSubmit:     return Submit(arguments);
    case kTPTSelectorWaitFence:  return WaitFence(arguments);
    case kTPTSelectorQueryInfo:  return QueryInfo(arguments);
    default:                     return kIOReturnBadArgument;
    }
}

/* Minimal ioctl stubs — full buffer management omitted for brevity;
 * would use IOBufferMemoryDescriptor + DMA-mapped pages per buffer. */

kern_return_t TPTUserClient::GemCreate(IOUserClientMethodArguments *args)
{
    if (args->scalarInputCount < 2) return kIOReturnBadArgument;
    uint64_t size  = args->scalarInput[0];
    uint64_t flags = args->scalarInput[1];
    /* TODO: allocate IOBufferMemoryDescriptor, pin, return handle */
    (void)size; (void)flags;
    args->scalarOutput[0] = 0; /* handle = 0 (placeholder) */
    return kIOReturnSuccess;
}

kern_return_t TPTUserClient::GemFree(IOUserClientMethodArguments *args)
{
    if (args->scalarInputCount < 1) return kIOReturnBadArgument;
    /* TODO: release buffer by handle */
    return kIOReturnSuccess;
}

kern_return_t TPTUserClient::GemInfo(IOUserClientMethodArguments *args)
{
    if (args->scalarInputCount < 1) return kIOReturnBadArgument;
    /* TODO: look up buffer, fill size + gpu_addr */
    args->scalarOutput[0] = 0; /* size */
    args->scalarOutput[1] = 0; /* gpu_addr */
    return kIOReturnSuccess;
}

kern_return_t TPTUserClient::Submit(IOUserClientMethodArguments *args)
{
    if (args->scalarInputCount < 3) return kIOReturnBadArgument;
    /* uint64_t cmdHandle = args->scalarInput[0];
       uint64_t offset    = args->scalarInput[1];
       uint64_t size      = args->scalarInput[2]; */
    fDriver->WriteReg32(kRegFenceEmit, (uint32_t)++args->scalarOutput[0]);
    return kIOReturnSuccess;
}

kern_return_t TPTUserClient::WaitFence(IOUserClientMethodArguments *args)
{
    if (args->scalarInputCount < 2) return kIOReturnBadArgument;
    /* Polling spin — replace with IODispatchSemaphore in production. */
    uint64_t seqno      = args->scalarInput[0];
    uint64_t timeoutNs  = args->scalarInput[1];
    uint64_t deadline   = clock_gettime_nsec_np(CLOCK_MONOTONIC_RAW) + timeoutNs;
    while (true) {
        uint32_t done = fDriver->ReadReg32(kRegFenceSeqno);
        if (done >= seqno) return kIOReturnSuccess;
        if (clock_gettime_nsec_np(CLOCK_MONOTONIC_RAW) > deadline)
            return kIOReturnTimeout;
        IOSleep(1);
    }
}

kern_return_t TPTUserClient::QueryInfo(IOUserClientMethodArguments *args)
{
    if (args->scalarInputCount < 1) return kIOReturnBadArgument;
    uint64_t query = args->scalarInput[0];
    switch (query) {
    case 0x01: args->scalarOutput[0] = fDriver->ReadReg32(kRegVramSize); break;
    case 0x03: args->scalarOutput[0] = fDriver->ReadReg32(kRegNumWarps); break;
    case 0x04: args->scalarOutput[0] = fDriver->ReadReg32(kRegNumCtas);  break;
    case 0x06: args->scalarOutput[0] = fDriver->ReadReg32(kRegWarpLanes);break;
    default:   return kIOReturnBadArgument;
    }
    return kIOReturnSuccess;
}
