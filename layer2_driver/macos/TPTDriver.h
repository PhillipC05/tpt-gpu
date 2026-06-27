/*
 * TPTDriver.h — TPT GPU macOS DriverKit IOService subclass
 *
 * Targets: DriverKit 21+ (macOS 12+), entitlement com.apple.developer.driverkit
 * Build:   Xcode 14+, DEXT bundle target
 */

#pragma once

#include <DriverKit/IOService.h>
#include <DriverKit/IOMemoryMap.h>
#include <DriverKit/IOBufferMemoryDescriptor.h>
#include <DriverKit/IOUserClient.h>
#include <DriverKit/IOInterruptDispatchSource.h>
#include <PCIDriverKit/PCIDriverKit.h>

/* PCI IDs */
#define kTPTPCIVendorID  0x1A2E
#define kTPTPCIDeviceID  0x0001

/* BAR indices */
#define kTPTBar0Index    0
#define kTPTBar2Index    2

/* Register offsets */
#define kRegDeviceID     0x0000
#define kRegStatus       0x0008
#define kRegReset        0x000C
#define kRegVramSize     0x0020
#define kRegIrqStatus    0x0080
#define kRegIrqMask      0x0084
#define kRegIrqAck       0x0088
#define kRegSchedEnable  0x0100
#define kRegNumWarps     0x0104
#define kRegNumCtas      0x0108
#define kRegWarpLanes    0x010C
#define kRegFenceSeqno   0x0060
#define kRegFenceEmit    0x0064

#define kIrqFenceSignaled  0x00000001u
#define kIrqError          0x80000000u

class TPTDriver : public IOService
{
    OSDeclareDefaultStructors(TPTDriver);

public:
    /* IOService lifecycle */
    virtual bool     Start(IOService *provider) override;
    virtual void     Stop(IOService *provider)  override;
    virtual void     free()                     override;

    /* Register accessors (inline, BAR0-relative) */
    uint32_t ReadReg32(uint32_t offset);
    void     WriteReg32(uint32_t offset, uint32_t value);

    /* Interrupt handling */
    void HandleInterrupt();

private:
    IOPCIDevice                  *fPCIDevice      = nullptr;
    IOMemoryMap                  *fBar0Map         = nullptr;  /* register space */
    IOMemoryMap                  *fBar2Map         = nullptr;  /* VRAM aperture  */
    IOInterruptDispatchSource    *fInterruptSource = nullptr;
    OSAction                     *fInterruptAction = nullptr;

    /* Fence state (access protected by lock) */
    IOLock                       *fFenceLock       = nullptr;
    uint64_t                      fNextSeqno       = 1;
    uint64_t                      fCompletedSeqno  = 0;
    bool                          fHardwareError   = false;
};

/* -------------------------------------------------------------------------
 * TPTUserClient — handles open() from userspace and dispatches ioctls
 * ---------------------------------------------------------------------- */
class TPTUserClient : public IOUserClient
{
    OSDeclareDefaultStructors(TPTUserClient);

public:
    virtual bool     Start(IOService *provider) override;
    virtual void     Stop(IOService *provider)  override;
    virtual void     free()                     override;

    /* External methods (called from userspace via IOConnectCallMethod) */
    virtual kern_return_t ExternalMethod(
        uint64_t              selector,
        IOUserClientMethodArguments *arguments,
        const IOUserClientMethodDispatch *dispatch,
        OSObject              *target,
        void                  *reference) override;

private:
    TPTDriver *fDriver = nullptr;

    kern_return_t GemCreate(IOUserClientMethodArguments *args);
    kern_return_t GemFree  (IOUserClientMethodArguments *args);
    kern_return_t GemInfo  (IOUserClientMethodArguments *args);
    kern_return_t Submit   (IOUserClientMethodArguments *args);
    kern_return_t WaitFence(IOUserClientMethodArguments *args);
    kern_return_t QueryInfo(IOUserClientMethodArguments *args);
};
