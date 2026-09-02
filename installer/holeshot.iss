#ifndef MyAppVersion
#define MyAppVersion "0.6.0"
#endif
#ifndef SourceDir
#define SourceDir "."
#endif
#ifndef OutputDir
#define OutputDir "..\dist"
#endif

#define MyAppName "Holeshot HUD"
#define MyAppPublisher "Holeshot HUD"
#define MyAppExeName "Holeshot-HUD.exe"
#define MyAppURL "https://github.com/LeadingTrendTechnologies/HoleshotHUD"

[Setup]
AppId={{A7C4E2B1-9F18-4D3A-8C21-6B4E9F2A1C08}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
DefaultDirName={localappdata}\{#MyAppName}
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
DisableDirPage=no
UsePreviousAppDir=yes
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
OutputDir={#OutputDir}
OutputBaseFilename=HoleshotHUD-Setup
Compression=lzma
SolidCompression=yes
WizardStyle=modern
SetupIconFile=..\overlay\icon.ico
UninstallDisplayIcon={app}\{#MyAppExeName}
UninstallDisplayName={#MyAppName}
SetupLogging=yes

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; GroupDescription: "Shortcuts:"; Flags: checkedonce

[Files]
Source: "{#SourceDir}\Holeshot-HUD.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\Holeshot-HUD.dlo"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\Install-Plugin.ps1"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\Uninstall.ps1"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\Uninstall.bat"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\README.txt"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"
Name: "{group}\Uninstall {#MyAppName}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "Start {#MyAppName}"; Flags: nowait postinstall skipifsilent

[UninstallRun]
Filename: "powershell.exe"; Parameters: "-NoProfile -ExecutionPolicy Bypass -File ""{app}\Uninstall.ps1"" -Silent"; Flags: runhidden waituntilterminated; RunOnceId: "RemovePlugin"

; Wipe leftovers Inno did not install (settings, game folder path, logs) and the app folder itself.
[UninstallDelete]
Type: filesandordirs; Name: "{app}"
Type: filesandordirs; Name: "{localappdata}\MXBO Overlay"
Type: files; Name: "{userdocs}\PiBoSo\MX Bikes\Holeshot-HUD.ini"
Type: files; Name: "{userdocs}\PiBoSo\MX Bikes\mxbo.ini"

[Code]
function RunPluginInstall(const GameDir: String): Integer;
var
  Args: String;
  ResultCode: Integer;
begin
  Args := '-NoProfile -ExecutionPolicy Bypass -File "' + ExpandConstant('{app}\Install-Plugin.ps1') +
    '" -PluginSrc "' + ExpandConstant('{app}\Holeshot-HUD.dlo') + '"';
  if GameDir <> '' then
    Args := Args + ' -GameDir "' + GameDir + '"';
  if not Exec(ExpandConstant('{sys}\WindowsPowerShell\v1.0\powershell.exe'),
    Args, '', SW_HIDE, ewWaitUntilTerminated, ResultCode) then
  begin
    Result := 2;
    exit;
  end;
  Result := ResultCode;
end;

procedure CurStepChanged(CurStep: TSetupStep);
var
  Code: Integer;
  GameDir: String;
begin
  if CurStep <> ssPostInstall then
    exit;

  Code := RunPluginInstall('');
  if Code = 0 then
    exit;

  if Code = 1 then
  begin
    GameDir := '';
    if BrowseForFolder('MX Bikes was not found. Select the MX Bikes folder (the one that contains mxbikes.exe).', GameDir, False) then
    begin
      Code := RunPluginInstall(GameDir);
      if Code = 0 then
        exit;
    end
    else
      exit;
  end;

  if Code = 3 then
    MsgBox('That folder does not look like MX Bikes (missing mxbikes.exe / plugins).', mbError, MB_OK)
  else
    MsgBox('Could not copy Holeshot-HUD.dlo into MX Bikes. Fully quit the game and run Setup again.', mbError, MB_OK);
end;
