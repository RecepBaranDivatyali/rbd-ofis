using System;
using System.IO;
using RbdOfis.Interop.Models;
using RbdOfis.Interop.Native;
using RbdOfis.Interop.Services;
using Xunit;

namespace RbdOfis.Windows.Tests;

public class PageOperationServiceTests : IDisposable
{
    private readonly string _tempInput;
    private readonly string _tempInput2;
    private readonly string _tempOutput;
    private readonly MockNativeRbdApi _mockApi;
    private readonly PageOperationService _service;

    public PageOperationServiceTests()
    {
        _tempInput = Path.GetTempFileName();
        _tempInput2 = Path.GetTempFileName();
        _tempOutput = Path.Combine(Path.GetTempPath(), $"out_{Guid.NewGuid():N}.pdf");

        File.WriteAllText(_tempInput, "%PDF-1.4 mock data");
        File.WriteAllText(_tempInput2, "%PDF-1.4 mock data 2");

        _mockApi = new MockNativeRbdApi();
        _service = new PageOperationService(_mockApi);
    }

    public void Dispose()
    {
        if (File.Exists(_tempInput)) File.Delete(_tempInput);
        if (File.Exists(_tempInput2)) File.Delete(_tempInput2);
        if (File.Exists(_tempOutput)) File.Delete(_tempOutput);
    }

    [Fact]
    public void Verify_Rotate_Calls_Api_And_Validates_Multiple_Of_90()
    {
        _service.Rotate(_tempInput, 90, _tempOutput);
        Assert.True(File.Exists(_tempOutput));

        Assert.Throws<ArgumentException>(() => _service.Rotate(_tempInput, 45, _tempOutput));
    }

    [Fact]
    public void Verify_DeletePages_Validates_NonEmpty_List()
    {
        _service.DeletePages(_tempInput, new[] { 1, 3 }, _tempOutput);
        Assert.True(File.Exists(_tempOutput));

        Assert.Throws<ArgumentException>(() => _service.DeletePages(_tempInput, Array.Empty<int>(), _tempOutput));
    }

    [Fact]
    public void Verify_Split_Validates_Page_Ranges()
    {
        _service.Split(_tempInput, 1, 5, _tempOutput);
        Assert.True(File.Exists(_tempOutput));

        Assert.Throws<ArgumentException>(() => _service.Split(_tempInput, 0, 5, _tempOutput));
        Assert.Throws<ArgumentException>(() => _service.Split(_tempInput, 6, 2, _tempOutput));
    }

    [Fact]
    public void Verify_Merge_Requires_At_Least_Two_Files()
    {
        _service.Merge(new[] { _tempInput, _tempInput2 }, _tempOutput);
        Assert.True(File.Exists(_tempOutput));

        Assert.Throws<ArgumentException>(() => _service.Merge(new[] { _tempInput }, _tempOutput));
    }

    [Fact]
    public void Verify_ReorderPages_Calls_Api()
    {
        _service.ReorderPages(_tempInput, new uint[] { 3, 1, 2 }, _tempOutput);
        Assert.True(File.Exists(_tempOutput));

        Assert.Throws<ArgumentException>(() => _service.ReorderPages(_tempInput, Array.Empty<uint>(), _tempOutput));
    }

    [Fact]
    public void Verify_CropPage_And_InsertBlankPage_Validate_Inputs()
    {
        _service.CropPage(_tempInput, 1, new PdfRect(10, 10, 200, 200), _tempOutput);
        Assert.True(File.Exists(_tempOutput));

        _service.InsertBlankPage(_tempInput, 1, 595.0f, 842.0f, _tempOutput);
        Assert.True(File.Exists(_tempOutput));

        Assert.Throws<ArgumentException>(() => _service.InsertBlankPage(_tempInput, 1, -100, 842.0f, _tempOutput));
    }

    [Fact]
    public void Verify_Missing_Input_File_Throws_FileNotFoundException()
    {
        string missingPath = Path.Combine(Path.GetTempPath(), "non_existent_file.pdf");
        Assert.Throws<FileNotFoundException>(() => _service.Rotate(missingPath, 90, _tempOutput));
    }
}
