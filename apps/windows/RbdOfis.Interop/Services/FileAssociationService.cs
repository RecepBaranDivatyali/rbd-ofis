using System;
using System.IO;
using System.Runtime.InteropServices;
using System.Runtime.Versioning;
using Microsoft.Win32;

namespace RbdOfis.Interop.Services;

/// <summary>
/// Service for managing Windows User-Level (HKCU) file associations for PDF files without requiring administrative privileges.
/// </summary>
[SupportedOSPlatform("windows")]
public static partial class FileAssociationService
{
    public const string ProgId = "RbdOfis.Pdf";
    public const string FileExtension = ".pdf";
    public const string DocDescription = "RBD OFİS PDF Document";

    private const int SHCNE_ASSOCCHANGED = 0x08000000;
    private const uint SHCNF_IDLIST = 0x0000;

    [LibraryImport("shell32.dll", EntryPoint = "SHChangeNotify")]
    private static partial void SHChangeNotify(int wEventId, uint uFlags, IntPtr dwItem1, IntPtr dwItem2);

    /// <summary>
    /// Notifies Windows Shell that file associations have changed.
    /// </summary>
    public static void NotifyShellAssociationChanged()
    {
        if (OperatingSystem.IsWindows())
        {
            try
            {
                SHChangeNotify(SHCNE_ASSOCCHANGED, SHCNF_IDLIST, IntPtr.Zero, IntPtr.Zero);
            }
            catch
            {
                // Non-critical notification failure
            }
        }
    }

    /// <summary>
    /// Registers RBD OFİS as the user-level handler for .pdf files in HKCU\Software\Classes.
    /// Does not require UAC elevation.
    /// </summary>
    public static bool RegisterPdfAssociation(string exePath)
    {
        if (!OperatingSystem.IsWindows()) return false;
        if (string.IsNullOrWhiteSpace(exePath)) return false;

        string normalizedExe = Path.GetFullPath(exePath);

        try
        {
            // 1. HKCU\Software\Classes\.pdf
            using (var extKey = Registry.CurrentUser.CreateSubKey(@"Software\Classes\" + FileExtension))
            {
                if (extKey != null)
                {
                    extKey.SetValue(string.Empty, ProgId);

                    using var openWithKey = extKey.CreateSubKey("OpenWithProgids");
                    openWithKey?.SetValue(ProgId, string.Empty);
                }
            }

            // 2. HKCU\Software\Classes\RbdOfis.Pdf
            using (var progIdKey = Registry.CurrentUser.CreateSubKey(@"Software\Classes\" + ProgId))
            {
                if (progIdKey != null)
                {
                    progIdKey.SetValue(string.Empty, DocDescription);

                    using var iconKey = progIdKey.CreateSubKey("DefaultIcon");
                    iconKey?.SetValue(string.Empty, $"\"{normalizedExe}\",0");

                    using var commandKey = progIdKey.CreateSubKey(@"shell\open\command");
                    commandKey?.SetValue(string.Empty, $"\"{normalizedExe}\" \"%1\"");
                }
            }

            NotifyShellAssociationChanged();
            return true;
        }
        catch
        {
            return false;
        }
    }

    /// <summary>
    /// Unregisters the user-level association for RBD OFİS.
    /// </summary>
    public static bool UnregisterPdfAssociation()
    {
        if (!OperatingSystem.IsWindows()) return false;

        try
        {
            // Remove ProgId
            Registry.CurrentUser.DeleteSubKeyTree(@"Software\Classes\" + ProgId, throwOnMissingSubKey: false);

            // Clear .pdf association if it points to RbdOfis.Pdf
            using (var extKey = Registry.CurrentUser.OpenSubKey(@"Software\Classes\" + FileExtension, writable: true))
            {
                if (extKey != null)
                {
                    var currentVal = extKey.GetValue(string.Empty) as string;
                    if (string.Equals(currentVal, ProgId, StringComparison.OrdinalIgnoreCase))
                    {
                        extKey.DeleteValue(string.Empty, throwOnMissingValue: false);
                    }

                    using var openWithKey = extKey.OpenSubKey("OpenWithProgids", writable: true);
                    openWithKey?.DeleteValue(ProgId, throwOnMissingValue: false);
                }
            }

            NotifyShellAssociationChanged();
            return true;
        }
        catch
        {
            return false;
        }
    }

    /// <summary>
    /// Checks whether .pdf is currently associated with RBD OFİS for the current user.
    /// </summary>
    public static bool IsPdfAssociated(string exePath)
    {
        if (!OperatingSystem.IsWindows()) return false;

        try
        {
            using var progIdKey = Registry.CurrentUser.OpenSubKey(@"Software\Classes\" + ProgId + @"\shell\open\command");
            if (progIdKey == null) return false;

            var cmd = progIdKey.GetValue(string.Empty) as string;
            if (string.IsNullOrWhiteSpace(cmd)) return false;

            if (!string.IsNullOrWhiteSpace(exePath))
            {
                string normalizedExe = Path.GetFullPath(exePath);
                if (!cmd.Contains(normalizedExe, StringComparison.OrdinalIgnoreCase) &&
                    !cmd.Contains(Path.GetFileName(normalizedExe), StringComparison.OrdinalIgnoreCase))
                {
                    return false;
                }
            }

            using var extKey = Registry.CurrentUser.OpenSubKey(@"Software\Classes\" + FileExtension);
            if (extKey == null) return false;

            var currentVal = extKey.GetValue(string.Empty) as string;
            return string.Equals(currentVal, ProgId, StringComparison.OrdinalIgnoreCase);
        }
        catch
        {
            return false;
        }
    }
}
