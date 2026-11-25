; Stosup Windows 64-bit インストーラースクリプト (Inno Setup)
; ビルド例: iscc /DMyAppVersion=0.1.0 installer\stosup.iss

#ifndef MyAppVersion
  #define MyAppVersion "0.1.0"
#endif

#define MyAppName "Stosup"
#define MyAppPublisher "tako-kun1"
#define MyAppURL "https://github.com/tako-kun1/stosup"
#define MyAppExeName "stosup.exe"
#define MyBuildDir "..\build\windows\x64\runner\Release"

[Setup]
; 固定GUID（上書きアップデートで同一製品として認識するため変更禁止）
AppId={{8F4C9E2A-7B31-4D56-9A2E-1C6F0B8D4E73}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppVerName={#MyAppName} {#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}/issues
AppUpdatesURL={#MyAppURL}/releases
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
; 管理者権限不要でインストール・自動アップデートを可能にする
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
; Windows 10 64bit 以降専用
MinVersion=10.0
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir=..\dist
OutputBaseFilename=Stosup-Setup-v{#MyAppVersion}
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
; 自動アップデート時に実行中のアプリを安全に終了する
CloseApplications=yes
CloseApplicationsFilter=stosup.exe
RestartApplications=no
SetupIconFile=..\windows\runner\resources\app_icon.ico
UninstallDisplayIcon={app}\{#MyAppExeName}

[Languages]
Name: "japanese"; MessagesFile: "compiler:Languages\Japanese.isl"
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "{#MyBuildDir}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{autoprograms}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon

[Run]
; 通常インストール後の起動オプション
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchProgram,{#StringChange(MyAppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent
; 自動アップデート（/SILENT /AUTOUPDATE 引数付き）完了後の自動再起動
Filename: "{app}\{#MyAppExeName}"; Parameters: "--after-update"; Flags: nowait skipifnotsilent; Check: IsAutoUpdateMode

[Code]
function IsAutoUpdateMode: Boolean;
var
  I: Integer;
begin
  Result := False;
  for I := 1 to ParamCount do
  begin
    if CompareText(ParamStr(I), '/AUTOUPDATE') = 0 then
    begin
      Result := True;
      Exit;
    end;
  end;
end;
