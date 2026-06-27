/*
 * tpt_wdm.c — TPT GPU Windows WDM kernel driver
 *
 * Implements: DriverEntry, AddDevice, PnP dispatch, power dispatch,
 * device control (ioctls), interrupt service routine (ISR), and DPC.
 *
 * Compile: WDK 10.0.22621.0, /kernel, x64
 * Sign:    EV certificate + cross-signed with Microsoft cross-cert chain
 */

#include "tpt_wdm.h"

/* =========================================================================
 * Forward declarations
 * ====================================================================== */
DRIVER_INITIALIZE           DriverEntry;
DRIVER_ADD_DEVICE           TptAddDevice;
__drv_dispatchType(IRP_MJ_PNP)
DRIVER_DISPATCH             TptDispatchPnp;
__drv_dispatchType(IRP_MJ_POWER)
DRIVER_DISPATCH             TptDispatchPower;
__drv_dispatchType(IRP_MJ_CREATE)
DRIVER_DISPATCH             TptDispatchCreate;
__drv_dispatchType(IRP_MJ_CLOSE)
DRIVER_DISPATCH             TptDispatchClose;
__drv_dispatchType(IRP_MJ_DEVICE_CONTROL)
DRIVER_DISPATCH             TptDispatchDeviceControl;
DRIVER_UNLOAD               TptDriverUnload;

KSERVICE_ROUTINE            TptInterruptService;
KDEFERRED_ROUTINE           TptFenceCompleteDpc;

/* =========================================================================
 * DriverEntry
 * ====================================================================== */
NTSTATUS DriverEntry(
    _In_ PDRIVER_OBJECT  DriverObject,
    _In_ PUNICODE_STRING RegistryPath)
{
    UNREFERENCED_PARAMETER(RegistryPath);

    DriverObject->DriverExtension->AddDevice       = TptAddDevice;
    DriverObject->DriverUnload                     = TptDriverUnload;
    DriverObject->MajorFunction[IRP_MJ_PNP]        = TptDispatchPnp;
    DriverObject->MajorFunction[IRP_MJ_POWER]      = TptDispatchPower;
    DriverObject->MajorFunction[IRP_MJ_CREATE]     = TptDispatchCreate;
    DriverObject->MajorFunction[IRP_MJ_CLOSE]      = TptDispatchClose;
    DriverObject->MajorFunction[IRP_MJ_DEVICE_CONTROL] = TptDispatchDeviceControl;

    KdPrintEx((DPFLTR_IHVDRIVER_ID, DPFLTR_INFO_LEVEL,
               "TPT GPU: DriverEntry\n"));
    return STATUS_SUCCESS;
}

VOID TptDriverUnload(_In_ PDRIVER_OBJECT DriverObject)
{
    UNREFERENCED_PARAMETER(DriverObject);
    KdPrintEx((DPFLTR_IHVDRIVER_ID, DPFLTR_INFO_LEVEL,
               "TPT GPU: DriverUnload\n"));
}

/* =========================================================================
 * AddDevice — attach FDO to the PDO supplied by PnP manager
 * ====================================================================== */
NTSTATUS TptAddDevice(
    _In_ PDRIVER_OBJECT DriverObject,
    _In_ PDEVICE_OBJECT PhysicalDeviceObject)
{
    PDEVICE_OBJECT    fdo;
    PTPT_DEVICE_EXT   ext;
    NTSTATUS          status;

    status = IoCreateDevice(
        DriverObject,
        sizeof(TPT_DEVICE_EXT),
        NULL,                       /* no name; use IoRegisterDeviceInterface */
        FILE_DEVICE_UNKNOWN,
        FILE_DEVICE_SECURE_OPEN,
        FALSE,
        &fdo);

    if (!NT_SUCCESS(status)) return status;

    ext = (PTPT_DEVICE_EXT)fdo->DeviceExtension;
    RtlZeroMemory(ext, sizeof(*ext));

    ext->PhysicalDeviceObject = PhysicalDeviceObject;
    ext->NextLowerDevice      = IoAttachDeviceToDeviceStack(fdo, PhysicalDeviceObject);
    if (!ext->NextLowerDevice) {
        IoDeleteDevice(fdo);
        return STATUS_NO_SUCH_DEVICE;
    }

    KeInitializeSpinLock(&ext->BufferLock);
    InitializeListHead(&ext->BufferList);
    ext->NextHandle = 1;

    KeInitializeSpinLock(&ext->FenceLock);
    KeInitializeEvent(&ext->FenceEvent, SynchronizationEvent, FALSE);
    ext->NextSeqno     = 1;
    ext->CompletedSeqno = 0;

    KeInitializeDpc(&ext->FenceCompleteDpc, TptFenceCompleteDpc, ext);

    /* Register device interface so userspace can open \\.\TPT_GPU0 */
    status = IoRegisterDeviceInterface(
        PhysicalDeviceObject,
        &GUID_TPT_GPU_INTERFACE,
        NULL,
        &ext->SymbolicLinkName);

    if (!NT_SUCCESS(status)) {
        IoDetachDevice(ext->NextLowerDevice);
        IoDeleteDevice(fdo);
        return status;
    }

    fdo->Flags &= ~DO_DEVICE_INITIALIZING;
    return STATUS_SUCCESS;
}

/* =========================================================================
 * PnP dispatch — handle IRP_MN_START_DEVICE / IRP_MN_REMOVE_DEVICE
 * ====================================================================== */
NTSTATUS TptDispatchPnp(
    _In_ PDEVICE_OBJECT DeviceObject,
    _Inout_ PIRP Irp)
{
    PTPT_DEVICE_EXT     ext    = DeviceObject->DeviceExtension;
    PIO_STACK_LOCATION  stack  = IoGetCurrentIrpStackLocation(Irp);
    NTSTATUS            status = STATUS_SUCCESS;

    switch (stack->MinorFunction) {

    case IRP_MN_START_DEVICE: {
        /* Forward to lower driver and wait for completion. */
        IoCopyCurrentIrpStackLocationToNext(Irp);
        KEVENT event;
        KeInitializeEvent(&event, NotificationEvent, FALSE);
        IoSetCompletionRoutine(Irp,
            [](PDEVICE_OBJECT, PIRP, PVOID ctx) -> NTSTATUS {
                KeSetEvent((PKEVENT)ctx, IO_NO_INCREMENT, FALSE);
                return STATUS_MORE_PROCESSING_REQUIRED;
            }, &event, TRUE, TRUE, TRUE);
        IoCallDriver(ext->NextLowerDevice, Irp);
        KeWaitForSingleObject(&event, Executive, KernelMode, FALSE, NULL);

        /* Map BAR0. */
        PCM_PARTIAL_RESOURCE_LIST resList =
            &stack->Parameters.StartDevice.AllocatedResourcesTranslated
             ->List[0].PartialResourceList;

        for (ULONG i = 0; i < resList->Count; i++) {
            PCM_PARTIAL_RESOURCE_DESCRIPTOR r = &resList->PartialDescriptors[i];
            if (r->Type == CmResourceTypeMemory) {
                if (ext->Bar0Va == NULL) {
                    ext->Bar0Pa   = r->u.Memory.Start;
                    ext->Bar0Size = r->u.Memory.Length;
                    ext->Bar0Va   = MmMapIoSpace(ext->Bar0Pa, ext->Bar0Size,
                                                 MmNonCached);
                } else if (ext->Bar2Va == NULL) {
                    ext->Bar2Pa   = r->u.Memory.Start;
                    ext->Bar2Size = r->u.Memory.Length;
                    ext->Bar2Va   = MmMapIoSpace(ext->Bar2Pa, ext->Bar2Size,
                                                 MmNonCached);
                }
            }
            if (r->Type == CmResourceTypeInterrupt) {
                IoConnectInterrupt(
                    &ext->InterruptObject,
                    TptInterruptService,
                    ext,
                    NULL,
                    r->u.Interrupt.Vector,
                    (KIRQL)r->u.Interrupt.Level,
                    (KIRQL)r->u.Interrupt.Level,
                    (r->Flags & CM_RESOURCE_INTERRUPT_LATCHED)
                        ? Latched : LevelSensitive,
                    TRUE,   /* shared */
                    r->u.Interrupt.Affinity,
                    FALSE);
            }
        }

        /* Enable interrupt mask, scheduler. */
        TptWriteReg32(ext, REG_IRQ_MASK, IRQ_FENCE_SIGNALED | IRQ_ERROR);
        TptWriteReg32(ext, REG_SCHED_ENABLE, 1);

        IoSetDeviceInterfaceState(&ext->SymbolicLinkName, TRUE);
        ext->InterfaceRegistered = TRUE;

        Irp->IoStatus.Status = STATUS_SUCCESS;
        IoCompleteRequest(Irp, IO_NO_INCREMENT);
        return STATUS_SUCCESS;
    }

    case IRP_MN_REMOVE_DEVICE:
        if (ext->InterfaceRegistered)
            IoSetDeviceInterfaceState(&ext->SymbolicLinkName, FALSE);
        if (ext->InterruptObject)
            IoDisconnectInterrupt(ext->InterruptObject);
        if (ext->Bar0Va)
            MmUnmapIoSpace(ext->Bar0Va, ext->Bar0Size);
        if (ext->Bar2Va)
            MmUnmapIoSpace(ext->Bar2Va, ext->Bar2Size);
        RtlFreeUnicodeString(&ext->SymbolicLinkName);
        IoSkipCurrentIrpStackLocation(Irp);
        status = IoCallDriver(ext->NextLowerDevice, Irp);
        IoDetachDevice(ext->NextLowerDevice);
        IoDeleteDevice(DeviceObject);
        return status;

    default:
        IoSkipCurrentIrpStackLocation(Irp);
        return IoCallDriver(ext->NextLowerDevice, Irp);
    }
}

/* =========================================================================
 * Power dispatch — pass-through for now
 * ====================================================================== */
NTSTATUS TptDispatchPower(
    _In_ PDEVICE_OBJECT DeviceObject,
    _Inout_ PIRP Irp)
{
    PTPT_DEVICE_EXT ext = DeviceObject->DeviceExtension;
    PoStartNextPowerIrp(Irp);
    IoSkipCurrentIrpStackLocation(Irp);
    return PoCallDriver(ext->NextLowerDevice, Irp);
}

/* =========================================================================
 * Create / Close
 * ====================================================================== */
NTSTATUS TptDispatchCreate(PDEVICE_OBJECT DevObj, PIRP Irp)
{
    UNREFERENCED_PARAMETER(DevObj);
    Irp->IoStatus.Status      = STATUS_SUCCESS;
    Irp->IoStatus.Information = 0;
    IoCompleteRequest(Irp, IO_NO_INCREMENT);
    return STATUS_SUCCESS;
}

NTSTATUS TptDispatchClose(PDEVICE_OBJECT DevObj, PIRP Irp)
{
    UNREFERENCED_PARAMETER(DevObj);
    Irp->IoStatus.Status      = STATUS_SUCCESS;
    Irp->IoStatus.Information = 0;
    IoCompleteRequest(Irp, IO_NO_INCREMENT);
    return STATUS_SUCCESS;
}

/* =========================================================================
 * Device control (ioctls)
 * ====================================================================== */
NTSTATUS TptDispatchDeviceControl(
    _In_ PDEVICE_OBJECT DeviceObject,
    _Inout_ PIRP Irp)
{
    PTPT_DEVICE_EXT     ext   = DeviceObject->DeviceExtension;
    PIO_STACK_LOCATION  stack = IoGetCurrentIrpStackLocation(Irp);
    NTSTATUS            status;
    ULONG_PTR           info  = 0;

    PVOID buf    = Irp->AssociatedIrp.SystemBuffer;
    ULONG inLen  = stack->Parameters.DeviceIoControl.InputBufferLength;
    ULONG outLen = stack->Parameters.DeviceIoControl.OutputBufferLength;

    switch (stack->Parameters.DeviceIoControl.IoControlCode) {

    case IOCTL_TPT_GEM_CREATE: {
        if (inLen < sizeof(struct tpt_gem_create) ||
            outLen < sizeof(struct tpt_gem_create)) {
            status = STATUS_BUFFER_TOO_SMALL; break;
        }
        struct tpt_gem_create *args = buf;
        status = TptAllocateBuffer(ext, args->size, args->flags, &args->handle);
        if (NT_SUCCESS(status)) info = sizeof(*args);
        break;
    }

    case IOCTL_TPT_GEM_FREE: {
        if (inLen < sizeof(struct tpt_gem_free)) {
            status = STATUS_BUFFER_TOO_SMALL; break;
        }
        struct tpt_gem_free *args = buf;
        status = TptFreeBuffer(ext, args->handle);
        break;
    }

    case IOCTL_TPT_GEM_INFO: {
        if (inLen < sizeof(struct tpt_gem_info) ||
            outLen < sizeof(struct tpt_gem_info)) {
            status = STATUS_BUFFER_TOO_SMALL; break;
        }
        struct tpt_gem_info *args = buf;
        PTPT_BUFFER tbuf = TptLookupBuffer(ext, args->handle);
        if (!tbuf) { status = STATUS_INVALID_HANDLE; break; }
        args->size     = tbuf->Size;
        args->gpu_addr = tbuf->GpuAddress;
        status = STATUS_SUCCESS;
        info   = sizeof(*args);
        break;
    }

    case IOCTL_TPT_SUBMIT: {
        if (inLen < sizeof(struct tpt_submit) ||
            outLen < sizeof(struct tpt_submit)) {
            status = STATUS_BUFFER_TOO_SMALL; break;
        }
        struct tpt_submit *args = buf;
        PTPT_BUFFER cmdbuf = TptLookupBuffer(ext, args->cmd_handle);
        if (!cmdbuf) { status = STATUS_INVALID_HANDLE; break; }

        KIRQL oldIrql;
        KeAcquireSpinLock(&ext->FenceLock, &oldIrql);
        ULONGLONG seqno = ext->NextSeqno++;
        KeReleaseSpinLock(&ext->FenceLock, oldIrql);

        TptWriteReg32(ext, REG_FENCE_EMIT, (ULONG)seqno);
        args->fence_seqno = seqno;
        status = STATUS_SUCCESS;
        info   = sizeof(*args);
        break;
    }

    case IOCTL_TPT_WAIT_FENCE: {
        if (inLen < sizeof(struct tpt_wait_fence)) {
            status = STATUS_BUFFER_TOO_SMALL; break;
        }
        struct tpt_wait_fence *args = buf;
        LARGE_INTEGER timeout;
        PLARGE_INTEGER ptimeout = NULL;
        if (args->timeout_ns != UINT64_MAX) {
            /* Convert ns to 100-ns units (negative = relative) */
            timeout.QuadPart = -(LONGLONG)(args->timeout_ns / 100);
            ptimeout = &timeout;
        }
        /* Poll-wait on FenceEvent; ISR signals it on completion. */
        status = KeWaitForSingleObject(&ext->FenceEvent, UserRequest,
                                       KernelMode, FALSE, ptimeout);
        if (status == STATUS_TIMEOUT) { status = STATUS_IO_TIMEOUT; break; }
        KIRQL oldIrql;
        KeAcquireSpinLock(&ext->FenceLock, &oldIrql);
        BOOLEAN done = (ext->CompletedSeqno >= args->fence_seqno);
        KeReleaseSpinLock(&ext->FenceLock, oldIrql);
        status = done ? STATUS_SUCCESS : STATUS_IO_TIMEOUT;
        break;
    }

    case IOCTL_TPT_QUERY_INFO: {
        if (inLen < sizeof(struct tpt_query_info) ||
            outLen < sizeof(struct tpt_query_info)) {
            status = STATUS_BUFFER_TOO_SMALL; break;
        }
        struct tpt_query_info *args = buf;
        switch (args->query) {
        case TPT_QUERY_VRAM_SIZE:  args->value = TptReadReg32(ext, REG_VRAM_SIZE); break;
        case TPT_QUERY_NUM_WARPS:  args->value = TptReadReg32(ext, REG_NUM_WARPS); break;
        case TPT_QUERY_NUM_CTAS:   args->value = TptReadReg32(ext, REG_NUM_CTAS);  break;
        case TPT_QUERY_WARP_LANES: args->value = TptReadReg32(ext, REG_WARP_LANES);break;
        case TPT_QUERY_DRIVER_VER:
            args->value = ((UINT64)TPT_DRIVER_MAJOR << 16) | TPT_DRIVER_MINOR; break;
        default: status = STATUS_INVALID_PARAMETER; goto done;
        }
        status = STATUS_SUCCESS;
        info   = sizeof(*args);
        break;
    }

    default:
        status = STATUS_INVALID_DEVICE_REQUEST;
        break;
    }

done:
    Irp->IoStatus.Status      = status;
    Irp->IoStatus.Information = info;
    IoCompleteRequest(Irp, IO_NO_INCREMENT);
    return status;
}

/* =========================================================================
 * Interrupt Service Routine
 * ====================================================================== */
BOOLEAN TptInterruptService(
    _In_ PKINTERRUPT Interrupt,
    _In_ PVOID       ServiceContext)
{
    UNREFERENCED_PARAMETER(Interrupt);
    PTPT_DEVICE_EXT ext = (PTPT_DEVICE_EXT)ServiceContext;

    ULONG status = READ_REGISTER_ULONG((PULONG)((PUCHAR)ext->Bar0Va + REG_IRQ_STATUS));
    if (status == 0) return FALSE;

    WRITE_REGISTER_ULONG((PULONG)((PUCHAR)ext->Bar0Va + REG_IRQ_ACK), status);

    if (status & (IRQ_FENCE_SIGNALED | IRQ_ERROR)) {
        KeInsertQueueDpc(&ext->FenceCompleteDpc, (PVOID)(ULONG_PTR)status, NULL);
    }
    return TRUE;
}

/* =========================================================================
 * DPC — fence completion (runs at DISPATCH_LEVEL)
 * ====================================================================== */
VOID TptFenceCompleteDpc(
    _In_ PKDPC    Dpc,
    _In_ PVOID    Context,
    _In_ PVOID    Arg1,
    _In_ PVOID    Arg2)
{
    UNREFERENCED_PARAMETER(Dpc);
    UNREFERENCED_PARAMETER(Arg2);
    PTPT_DEVICE_EXT ext    = (PTPT_DEVICE_EXT)Context;
    ULONG           irqSts = (ULONG)(ULONG_PTR)Arg1;

    if (irqSts & IRQ_ERROR) {
        KdPrintEx((DPFLTR_IHVDRIVER_ID, DPFLTR_ERROR_LEVEL,
                   "TPT GPU: hardware error\n"));
        ext->HardwareError = TRUE;
    }

    if (irqSts & IRQ_FENCE_SIGNALED) {
        ULONG seqno = TptReadReg32(ext, REG_FENCE_SEQNO);
        KIRQL oldIrql;
        KeAcquireSpinLockAtDpcLevel(&ext->FenceLock);
        ext->CompletedSeqno = seqno;
        KeReleaseSpinLockFromDpcLevel(&ext->FenceLock);
        KeSetEvent(&ext->FenceEvent, IO_NO_INCREMENT, FALSE);
    }
}

/* =========================================================================
 * Register helpers
 * ====================================================================== */
ULONG TptReadReg32(PTPT_DEVICE_EXT ext, ULONG offset)
{
    return READ_REGISTER_ULONG(
        (PULONG)((PUCHAR)ext->Bar0Va + offset));
}

VOID TptWriteReg32(PTPT_DEVICE_EXT ext, ULONG offset, ULONG value)
{
    WRITE_REGISTER_ULONG(
        (PULONG)((PUCHAR)ext->Bar0Va + offset), value);
}

/* =========================================================================
 * Buffer management
 * ====================================================================== */
NTSTATUS TptAllocateBuffer(
    PTPT_DEVICE_EXT ext,
    UINT64          size,
    UINT32          flags,
    PULONG          outHandle)
{
    /* Page-align size. */
    SIZE_T alignedSize = (SIZE_T)((size + PAGE_SIZE - 1) & ~(PAGE_SIZE - 1));
    if (alignedSize == 0) return STATUS_INVALID_PARAMETER;

    PTPT_BUFFER buf = ExAllocatePoolWithTag(
        NonPagedPoolNx, sizeof(TPT_BUFFER), TPT_POOL_TAG);
    if (!buf) return STATUS_INSUFFICIENT_RESOURCES;
    RtlZeroMemory(buf, sizeof(*buf));

    /* Allocate physical pages and lock them. */
    buf->Mdl = MmAllocatePagesForMdl(
        (PHYSICAL_ADDRESS){.QuadPart = 0},
        (PHYSICAL_ADDRESS){.QuadPart = 0x7FFFFFFF},
        (PHYSICAL_ADDRESS){.QuadPart = 0},
        alignedSize);
    if (!buf->Mdl) {
        ExFreePoolWithTag(buf, TPT_POOL_TAG);
        return STATUS_INSUFFICIENT_RESOURCES;
    }

    buf->KernelVa = MmMapLockedPagesSpecifyCache(
        buf->Mdl, KernelMode, MmNonCached, NULL, FALSE, NormalPagePriority);
    if (!buf->KernelVa) {
        MmFreePagesFromMdl(buf->Mdl);
        ExFreePoolWithTag(buf, TPT_POOL_TAG);
        return STATUS_INSUFFICIENT_RESOURCES;
    }

    buf->Size  = alignedSize;
    buf->Flags = flags;

    /* Assign GPU address: for VRAM flag use BAR2, else GTT (physical). */
    if (flags & TPT_BUF_FLAG_VRAM) {
        buf->GpuAddress = ext->Bar2Pa.QuadPart;  /* simplified; real driver uses heap */
    } else {
        buf->GpuAddress = MmGetPhysicalAddress(buf->KernelVa).QuadPart;
    }

    KIRQL oldIrql;
    KeAcquireSpinLock(&ext->BufferLock, &oldIrql);
    buf->Handle = ext->NextHandle++;
    InsertTailList(&ext->BufferList, &buf->ListEntry);
    KeReleaseSpinLock(&ext->BufferLock, oldIrql);

    *outHandle = buf->Handle;
    return STATUS_SUCCESS;
}

NTSTATUS TptFreeBuffer(PTPT_DEVICE_EXT ext, ULONG handle)
{
    KIRQL oldIrql;
    KeAcquireSpinLock(&ext->BufferLock, &oldIrql);
    PTPT_BUFFER found = NULL;
    for (PLIST_ENTRY e = ext->BufferList.Flink;
         e != &ext->BufferList; e = e->Flink) {
        PTPT_BUFFER b = CONTAINING_RECORD(e, TPT_BUFFER, ListEntry);
        if (b->Handle == handle) { found = b; break; }
    }
    if (found) RemoveEntryList(&found->ListEntry);
    KeReleaseSpinLock(&ext->BufferLock, oldIrql);

    if (!found) return STATUS_INVALID_HANDLE;

    MmUnmapLockedPages(found->KernelVa, found->Mdl);
    MmFreePagesFromMdl(found->Mdl);
    ExFreePoolWithTag(found, TPT_POOL_TAG);
    return STATUS_SUCCESS;
}

PTPT_BUFFER TptLookupBuffer(PTPT_DEVICE_EXT ext, ULONG handle)
{
    KIRQL oldIrql;
    KeAcquireSpinLock(&ext->BufferLock, &oldIrql);
    PTPT_BUFFER found = NULL;
    for (PLIST_ENTRY e = ext->BufferList.Flink;
         e != &ext->BufferList; e = e->Flink) {
        PTPT_BUFFER b = CONTAINING_RECORD(e, TPT_BUFFER, ListEntry);
        if (b->Handle == handle) { found = b; break; }
    }
    KeReleaseSpinLock(&ext->BufferLock, oldIrql);
    return found;
}
