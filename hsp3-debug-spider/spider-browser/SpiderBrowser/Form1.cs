using CefSharp.WinForms;
using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.Data;
using System.Drawing;
using System.Linq;
using System.Text;
using System.Threading.Tasks;
using System.Windows.Forms;

namespace SpiderBrowser
{
    public partial class Form1 : Form
    {
        public Form1()
        {
            InitializeComponent();

            var browser = new ChromiumWebBrowser("http://localhost:8080/index.html");
            Controls.Add(browser);
            browser.Dock = DockStyle.Fill;
        }

        protected override void OnLoad(EventArgs e)
        {
            base.OnLoad(e);

            Width = 480;
            Height = Screen.PrimaryScreen.WorkingArea.Height;
            SetDesktopLocation(
                Screen.PrimaryScreen.WorkingArea.Right - Width,
                Screen.PrimaryScreen.WorkingArea.Top
            );
        }
    }
}
