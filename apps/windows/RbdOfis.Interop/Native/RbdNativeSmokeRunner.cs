using System;
using System.Diagnostics;
using System.IO;
using System.Runtime.InteropServices;

namespace RbdOfis.Interop.Native;

/// <summary>
/// Report structure for the native smoke test runner.
/// </summary>
public sealed class SmokeTestReport
{
    public bool IsSuccess { get; init; }
    public bool IsNativeDllLoaded { get; init; }
    public bool ZeroCopyVerified { get; init; }
    public long LatencyMicroseconds { get; init; }
    public int RenderedBytesCount { get; init; }
    public string Details { get; init; } = string.Empty;
}

/// <summary>
/// Self-contained native test/smoke runner.
/// Exercises the C-ABI interop, SafeHandle lifecycle, and zero-copy pinned buffer rendering.
/// </summary>
public static class RbdNativeSmokeRunner
{
    public static SmokeTestReport Run(bool forceMock = false)
    {
        var sw = Stopwatch.StartNew();
        bool isNative = false;
        INativeRbdApi api;

        if (!forceMock && TryCheckNativeDll(out string dllPath))
        {
            api = NativeRbdApi.Instance;
            isNative = true;
        }
        else
        {
            api = new MockNativeRbdApi();
            isNative = false;
        }

        try
        {
            using var engine = new RbdEngine(api);
            byte[] mockDocHeader = [0x25, 0x50, 0x44, 0x46, 0x2D, 0x31, 0x2E, 0x37]; // "%PDF-1.7"
            using var doc = engine.OpenDocument(mockDocHeader);

            int pageCount = doc.PageCount;
            if (pageCount <= 0)
            {
                return new SmokeTestReport
                {
                    IsSuccess = false,
                    IsNativeDllLoaded = isNative,
                    Details = $"Document opened but reported invalid page count: {pageCount}"
                };
            }

            const int tileWidth = 256;
            const int tileHeight = 256;
            const int stride = tileWidth * 4;
            byte[] tileBuffer = new byte[stride * tileHeight];

            // Execute zero-copy pinned buffer rendering
            doc.RenderTileIntoBuffer(
                pageIndex: 0,
                tileX: 0,
                tileY: 0,
                tileWidth: tileWidth,
                tileHeight: tileHeight,
                zoom: 1.0,
                destinationBuffer: tileBuffer,
                stride: stride,
                format: RbdPixelFormat.Bgra32);

            // Verify memory buffer received pixels
            int nonZeroCount = 0;
            for (int i = 0; i < tileBuffer.Length; i++)
            {
                if (tileBuffer[i] != 0)
                {
                    nonZeroCount++;
                }
            }

            sw.Stop();
            long microseconds = (long)(sw.Elapsed.TotalMilliseconds * 1000.0);

            bool zeroCopyOk = nonZeroCount > 0;

            return new SmokeTestReport
            {
                IsSuccess = zeroCopyOk,
                IsNativeDllLoaded = isNative,
                ZeroCopyVerified = zeroCopyOk,
                LatencyMicroseconds = microseconds,
                RenderedBytesCount = nonZeroCount,
                Details = isNative
                    ? $"Native rbd_ffi.dll executed successfully. {nonZeroCount} bytes verified in pinned memory."
                    : $"Self-contained mock native C-ABI executed successfully. {nonZeroCount} bytes verified in pinned memory."
            };
        }
        catch (Exception ex)
        {
            sw.Stop();
            return new SmokeTestReport
            {
                IsSuccess = false,
                IsNativeDllLoaded = isNative,
                Details = $"Smoke test failed with exception: {ex.Message}"
            };
        }
    }

    private static bool TryCheckNativeDll(out string foundPath)
    {
        string baseDir = AppContext.BaseDirectory;
        string[] candidates =
        [
            Path.Combine(baseDir, "rbd_ffi.dll"),
            Path.Combine(baseDir, "runtimes", "win-x64", "native", "rbd_ffi.dll"),
            Path.Combine(baseDir, "..", "..", "..", "..", "..", "core", "target", "release", "rbd_ffi.dll"),
            Path.Combine(baseDir, "..", "..", "..", "..", "..", "core", "target", "debug", "rbd_ffi.dll"),
            Path.Combine(baseDir, "..", "..", "..", "..", "..", "target", "release", "rbd_ffi.dll"),
            Path.Combine(baseDir, "..", "..", "..", "..", "..", "target", "debug", "rbd_ffi.dll")
        ];

        foreach (string candidate in candidates)
        {
            try
            {
                string full = Path.GetFullPath(candidate);
                if (File.Exists(full) && NativeLibrary.TryLoad(full, out IntPtr handle))
                {
                    NativeLibrary.Free(handle);
                    foundPath = full;
                    return true;
                }
            }
            catch
            {
                // Ignore load failures during probing
            }
        }

        foundPath = string.Empty;
        return false;
    }
}
