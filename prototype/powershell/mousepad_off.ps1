$csharp = @"
using System;
using System.Runtime.InteropServices;

public class HidDevice {
    [DllImport("hid.dll", SetLastError = true)]
    public static extern void HidD_GetHidGuid(out Guid hidGuid);

    [DllImport("setupapi.dll", SetLastError = true)]
    public static extern IntPtr SetupDiGetClassDevs(ref Guid ClassGuid, IntPtr Enumerator, IntPtr hwndParent, uint Flags);

    [DllImport("setupapi.dll", SetLastError = true)]
    public static extern bool SetupDiEnumDeviceInterfaces(IntPtr DeviceInfoSet, IntPtr DeviceInfoData, ref Guid InterfaceClassGuid, uint MemberIndex, ref SP_DEVICE_INTERFACE_DATA DeviceInterfaceData);

    [DllImport("setupapi.dll", SetLastError = true, CharSet = CharSet.Auto)]
    public static extern bool SetupDiGetDeviceInterfaceDetail(IntPtr DeviceInfoSet, ref SP_DEVICE_INTERFACE_DATA DeviceInterfaceData, IntPtr DeviceInterfaceDetailData, uint DeviceInterfaceDetailDataSize, out uint RequiredSize, IntPtr DeviceInfoData);

    [DllImport("kernel32.dll", SetLastError = true, CharSet = CharSet.Auto)]
    public static extern IntPtr CreateFile(string lpFileName, uint dwDesiredAccess, uint dwShareMode, IntPtr lpSecurityAttributes, uint dwCreationDisposition, uint dwFlagsAndAttributes, IntPtr hTemplateFile);

    [DllImport("kernel32.dll", SetLastError = true)]
    public static extern bool CloseHandle(IntPtr hObject);

    [DllImport("hid.dll", SetLastError = true)]
    public static extern bool HidD_SetFeature(IntPtr HidDeviceObject, byte[] ReportBuffer, uint ReportBufferLength);

    [StructLayout(LayoutKind.Sequential)]
    public struct SP_DEVICE_INTERFACE_DATA {
        public uint cbSize;
        public Guid InterfaceClassGuid;
        public uint Flags;
        public IntPtr Reserved;
    }

    public static void SendFeatureReport(string vidPid, byte[] report) {
        Guid hidGuid;
        HidD_GetHidGuid(out hidGuid);
        IntPtr deviceInfoSet = SetupDiGetClassDevs(ref hidGuid, IntPtr.Zero, IntPtr.Zero, 0x12);
        
        SP_DEVICE_INTERFACE_DATA ifaceData = new SP_DEVICE_INTERFACE_DATA();
        ifaceData.cbSize = (uint)Marshal.SizeOf(ifaceData);
        
        for (uint i = 0; SetupDiEnumDeviceInterfaces(deviceInfoSet, IntPtr.Zero, ref hidGuid, i, ref ifaceData); i++) {
            uint requiredSize;
            SetupDiGetDeviceInterfaceDetail(deviceInfoSet, ref ifaceData, IntPtr.Zero, 0, out requiredSize, IntPtr.Zero);
            IntPtr detailData = Marshal.AllocHGlobal((int)requiredSize);
            Marshal.WriteInt32(detailData, (IntPtr.Size == 8) ? 8 : 5);
            
            if (SetupDiGetDeviceInterfaceDetail(deviceInfoSet, ref ifaceData, detailData, requiredSize, out requiredSize, IntPtr.Zero)) {
                string path = Marshal.PtrToStringAuto(new IntPtr(detailData.ToInt64() + 4));
                if (path.Contains(vidPid)) {
                    IntPtr handle = CreateFile(path, 0xC0000000, 3, IntPtr.Zero, 3, 0, IntPtr.Zero);
                    if (handle != new IntPtr(-1)) {
                        HidD_SetFeature(handle, report, (uint)report.Length);
                        CloseHandle(handle);
                    }
                }
            }
            Marshal.FreeHGlobal(detailData);
        }
    }
}
"@

try {
    Add-Type -TypeDefinition $csharp
} catch {
    # Ignore error if type has already been added in this session
}

# Mode 0x04 = Off, Brightness = 0x00, Speed = 0x00
$packet_off = [byte[]]@([byte]0x00, [byte]0x08, [byte]0x04, [byte]0x00, [byte]0x00, [byte]0x00, [byte]0x00, [byte]0x00, [byte]0x00)

# Target device: Holtek/G-aim Mousepad (VID: 0x04D9, PID: 0xA1FA)
[HidDevice]::SendFeatureReport("vid_04d9&pid_a1fa", $packet_off)

# Hard terminate process so it does not hang Windows shutdown
Stop-Process -Id $PID -Force
