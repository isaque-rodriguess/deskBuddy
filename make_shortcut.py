"""make_shortcut.py — Cria atalho na área de trabalho + pasta Startup."""
import os
import sys
import ctypes

def create_shortcut():
    """Cria atalho .lnk na área de trabalho."""
    try:
        import winshell
        from winshell import desktop, startup
        from win32com.client import Dispatch

        desktop_path = desktop()
        startup_path = startup()

        python_exe = sys.executable.replace("python.exe", "pythonw.exe")
        script = os.path.join(os.path.dirname(os.path.abspath(__file__)), "pet.pyw")

        shell = Dispatch("WScript.Shell")

        # Atalho na área de trabalho
        shortcut = shell.CreateShortCut(os.path.join(desktop_path, "Desktop Pet.lnk"))
        shortcut.Targetpath = python_exe
        shortcut.Arguments = f'"{script}"'
        shortcut.WorkingDirectory = os.path.dirname(script)
        shortcut.IconLocation = python_exe
        shortcut.Description = "Desktop Pet Zero"
        shortcut.save()
        print(f"✓ Desktop shortcut: {desktop_path}")

        # Atalho no Startup (auto-start)
        shortcut2 = shell.CreateShortCut(os.path.join(startup_path, "Desktop Pet.lnk"))
        shortcut2.Targetpath = python_exe
        shortcut2.Arguments = f'"{script}"'
        shortcut2.WorkingDirectory = os.path.dirname(script)
        shortcut2.IconLocation = python_exe
        shortcut2.Description = "Desktop Pet Zero (auto-start)"
        shortcut2.save()
        print(f"✓ Startup shortcut: {startup_path}")

        return True
    except ImportError:
        # Fallback: criar .bat na área de trabalho
        desktop_path = os.path.join(os.path.expanduser("~"), "Desktop")
        bat_path = os.path.join(desktop_path, "Desktop Pet.bat")
        script = os.path.join(os.path.dirname(os.path.abspath(__file__)), "pet.pyw")

        with open(bat_path, "w") as f:
            f.write(f'@echo off\nstart "" pythonw "{script}"\n')

        print(f"✓ Desktop shortcut (bat): {bat_path}")
        print("  (Install pywin32 for proper .lnk shortcuts: pip install pywin32)")
        return True


if __name__ == "__main__":
    create_shortcut()
