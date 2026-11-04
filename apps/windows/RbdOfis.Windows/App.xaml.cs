using System;
using System.IO;
using System.Windows;
using RbdOfis.Interop.Services;

namespace RbdOfis.Windows;

/// <summary>
/// Interaction logic for App.xaml
/// </summary>
public partial class App : Application
{
    protected override void OnStartup(StartupEventArgs e)
    {
        base.OnStartup(e);

        string[] args = e.Args;
        string exePath = Environment.ProcessPath ?? Environment.GetCommandLineArgs()[0];

        foreach (string arg in args)
        {
            if (string.Equals(arg, "--register-associations", StringComparison.OrdinalIgnoreCase))
            {
                FileAssociationService.RegisterPdfAssociation(exePath);
                Shutdown(0);
                return;
            }

            if (string.Equals(arg, "--unregister-associations", StringComparison.OrdinalIgnoreCase))
            {
                FileAssociationService.UnregisterPdfAssociation();
                Shutdown(0);
                return;
            }
        }

        string? initialPdfPath = null;
        foreach (string arg in args)
        {
            if (!arg.StartsWith("--", StringComparison.OrdinalIgnoreCase) &&
                arg.EndsWith(".pdf", StringComparison.OrdinalIgnoreCase) &&
                File.Exists(arg))
            {
                initialPdfPath = Path.GetFullPath(arg);
                break;
            }
        }

        var mainWindow = new MainWindow(initialPdfPath);
        MainWindow = mainWindow;
        mainWindow.Show();
    }
}
