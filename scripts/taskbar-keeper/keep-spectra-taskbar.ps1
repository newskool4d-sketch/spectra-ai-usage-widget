$ErrorActionPreference='Stop'
$installed=Join-Path $env:LOCALAPPDATA 'SPECTRA\spectra-native.exe'
Add-Type -TypeDefinition @'
using System;using System.Runtime.InteropServices;
public static class SpectraTaskbarKeeper {
 [DllImport("user32.dll",CharSet=CharSet.Unicode)] public static extern IntPtr FindWindow(string c,string t);
 [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h,out uint p);
 [DllImport("user32.dll")] public static extern IntPtr GetWindow(IntPtr h,uint c);
 [DllImport("user32.dll",EntryPoint="SetWindowLongPtrW",SetLastError=true)] public static extern IntPtr SetOwner(IntPtr h,int n,IntPtr p);
 [DllImport("kernel32.dll")] public static extern void SetLastError(uint e);
 [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
 public static uint Pid(IntPtr h){uint p;GetWindowThreadProcessId(h,out p);return p;}
 public static string Keep(uint expected){var s=FindWindow("SpectraTaskbarStrip",null);var t=FindWindow("Shell_TrayWnd",null);if(s==IntPtr.Zero||t==IntPtr.Zero||Pid(s)!=expected)return "waiting";var owner=GetWindow(s,4);if(owner!=t){SetLastError(0);SetOwner(s,-8,t);owner=GetWindow(s,4);}return "pid="+expected+" strip="+s+" owner="+owner+" tray="+t+" visible="+IsWindowVisible(s);}
}
'@
$log=Join-Path $PSScriptRoot 'spectra-taskbar-keeper.log'
$last=''
Write-Output 'KEEPER_READY'
while($true){
 $process=@(Get-Process -Name 'spectra-native' -ErrorAction SilentlyContinue|Where-Object {$_.Path -eq $installed})
 if($process.Count -eq 0){break}
 if($process.Count -ne 1){throw 'Expected exactly one installed SPECTRA process'}
 $state=[SpectraTaskbarKeeper]::Keep([uint32]$process[0].Id)
 if($state -ne $last){$line=[DateTime]::Now.ToString('o')+' '+$state;Add-Content -LiteralPath $log -Value $line -Encoding utf8;Write-Output $line;$last=$state}
 Start-Sleep -Milliseconds 250
}
Write-Output 'KEEPER_STOPPED_APP_EXITED'
