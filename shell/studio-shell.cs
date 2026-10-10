// studio-shell.exe — the native desktop window for Creative Studio apps.
//
// A tiny WinForms + WebView2 host: any studio app is just a local URL, so this
// shell renders it in a real native window (taskbar identity, app icon, no
// browser chrome) instead of an Edge/Chrome --app= window.
//
// Compile (see scripts/build-shell.ps1):
//   csc.exe /target:winexe /out:assets\studio-shell.exe ^
//       /r:System.Windows.Forms.dll /r:System.Drawing.dll ^
//       /r:assets\webview2\Microsoft.Web.WebView2.Core.dll ^
//       /r:assets\webview2\Microsoft.Web.WebView2.WinForms.dll ^
//       shell\studio-shell.cs
//
// Usage:
//   studio-shell.exe <url> [--title "Name"] [--icon path.ico]
//                    [--user-data-dir dir] [--size 1440x900]
using System;
using System.Drawing;
using System.IO;
using System.Threading.Tasks;
using System.Windows.Forms;
using Microsoft.Web.WebView2.Core;
using Microsoft.Web.WebView2.WinForms;

namespace CreativeStudioShell
{
    internal static class Program
    {
        private static readonly string LogPath = Path.Combine(Path.GetTempPath(), "studio-shell.log");

        internal static void Log(string message)
        {
            try
            {
                string line = DateTime.Now.ToString("HH:mm:ss.fff") + " " + message + Environment.NewLine;
                File.AppendAllText(LogPath, line);
            }
            catch
            {
            }
        }

        [STAThread]
        private static void Main(string[] args)
        {
            Log("studio-shell starting, args: " + string.Join(" ", args));

            string url = "";
            string title = "Creative Studio";
            string iconPath = "";
            string userDataDir = "";
            int width = 1440;
            int height = 900;

            for (int i = 0; i < args.Length; i++)
            {
                string a = args[i];
                if (a == "--title" && i + 1 < args.Length)
                {
                    title = args[++i];
                }
                else if (a == "--icon" && i + 1 < args.Length)
                {
                    iconPath = args[++i];
                }
                else if (a == "--user-data-dir" && i + 1 < args.Length)
                {
                    userDataDir = args[++i];
                }
                else if (a == "--size" && i + 1 < args.Length)
                {
                    string[] parts = args[++i].Split('x');
                    int w = 0;
                    int h = 0;
                    if (parts.Length == 2 && int.TryParse(parts[0], out w) && int.TryParse(parts[1], out h)
                        && w > 200 && h > 200)
                    {
                        width = w;
                        height = h;
                    }
                }
                else if (string.IsNullOrEmpty(url))
                {
                    url = a;
                }
            }

            if (string.IsNullOrEmpty(url))
            {
                MessageBox.Show("usage: studio-shell.exe <url> [--title name] [--icon file.ico]"
                    + " [--user-data-dir dir] [--size 1440x900]", "studio-shell");
                return;
            }

            Log("parsed ok, title=" + title + " url=" + url + " userDataDir=" + userDataDir);
            try
            {
                Application.EnableVisualStyles();
                Application.SetCompatibleTextRenderingDefault(false);
                Application.Run(new ShellForm(url, title, iconPath, userDataDir, width, height));
            }
            catch (Exception ex)
            {
                Log("fatal: " + ex);
            }
        }
    }

    internal class ShellForm : Form
    {
        private readonly WebView2 _webView;
        private readonly string _url;
        private readonly string _userDataDir;

        public ShellForm(string url, string title, string iconPath, string userDataDir, int width, int height)
        {
            _url = url;
            _userDataDir = userDataDir;

            Text = title;
            Width = width;
            Height = height;
            MinimumSize = new Size(640, 480);
            StartPosition = FormStartPosition.CenterScreen;
            BackColor = Color.FromArgb(16, 20, 26);

            if (!string.IsNullOrEmpty(iconPath) && File.Exists(iconPath))
            {
                try
                {
                    Icon = new Icon(iconPath);
                    ShowIcon = true;
                }
                catch
                {
                    // keep the default icon
                }
            }

            _webView = new WebView2 { Dock = DockStyle.Fill };
            _webView.DefaultBackgroundColor = Color.FromArgb(16, 20, 26);
            Controls.Add(_webView);
        }

        protected override void OnShown(EventArgs e)
        {
            base.OnShown(e);
            Program.Log("form shown, starting WebView2");
            InitAsync();
        }

        private async void InitAsync()
        {
            try
            {
                Program.Log("CoreWebView2Environment.CreateAsync...");
                CoreWebView2Environment environment;
                if (!string.IsNullOrEmpty(_userDataDir))
                {
                    environment = await CoreWebView2Environment.CreateAsync(null, _userDataDir, null);
                }
                else
                {
                    environment = await CoreWebView2Environment.CreateAsync();
                }
                Program.Log("environment ready, EnsureCoreWebView2Async...");
                await _webView.EnsureCoreWebView2Async(environment);
                Program.Log("webview ready, navigating to " + _url);
                _webView.Source = new Uri(_url);
                _webView.Focus();
            }
            catch (Exception ex)
            {
                Program.Log("webview init failed: " + ex);
                Text = Text + " (WebView2 failed)";
            }
        }
    }
}