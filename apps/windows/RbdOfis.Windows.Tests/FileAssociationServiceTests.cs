using System;
using System.IO;
using System.Runtime.Versioning;
using RbdOfis.Interop.Services;
using Xunit;

namespace RbdOfis.Windows.Tests;

[SupportedOSPlatform("windows")]
public sealed class FileAssociationServiceTests
{
    [Fact]
    public void Verify_ProgId_And_Extension_Constants()
    {
        Assert.Equal("RbdOfis.Pdf", FileAssociationService.ProgId);
        Assert.Equal(".pdf", FileAssociationService.FileExtension);
    }

    [Fact]
    public void Verify_Invalid_Arguments_Return_False()
    {
        Assert.False(FileAssociationService.RegisterPdfAssociation(""));
        Assert.False(FileAssociationService.RegisterPdfAssociation("   "));
        Assert.False(FileAssociationService.RegisterPdfAssociation(null!));
    }

    [Fact]
    public void Verify_NotifyShellAssociationChanged_Executes_Safely()
    {
        var ex = Record.Exception(() => FileAssociationService.NotifyShellAssociationChanged());
        Assert.Null(ex);
    }

    [Fact]
    public void Verify_Register_And_Unregister_Lifecycle()
    {
        if (!OperatingSystem.IsWindows())
        {
            return;
        }

        string tempExePath = Path.Combine(Path.GetTempPath(), "RbdOfis_Test_Fake.exe");

        try
        {
            bool registered = FileAssociationService.RegisterPdfAssociation(tempExePath);
            Assert.True(registered);

            bool isAssociated = FileAssociationService.IsPdfAssociated(tempExePath);
            Assert.True(isAssociated);

            bool unregistered = FileAssociationService.UnregisterPdfAssociation();
            Assert.True(unregistered);

            bool isAssociatedAfter = FileAssociationService.IsPdfAssociated(tempExePath);
            Assert.False(isAssociatedAfter);
        }
        finally
        {
            // Ensure cleaned up
            FileAssociationService.UnregisterPdfAssociation();
        }
    }
}
