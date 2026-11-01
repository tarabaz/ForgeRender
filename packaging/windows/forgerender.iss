; Installer di ForgeRender (Inno Setup 6).
; Compilazione:  ISCC.exe /DAppVersion=0.26.0 /DBinDir=..\..\target\release packaging\windows\forgerender.iss

#ifndef AppVersion
  #define AppVersion "0.26.0"
#endif
#ifndef BinDir
  #define BinDir "..\..\target\release"
#endif

[Setup]
AppId={{8F3C2A51-6B7D-4E0A-9C1F-2D5E7A9B3C41}
AppName=ForgeRender
AppVersion={#AppVersion}
AppPublisher=FrancyStore3D
DefaultDirName={autopf}\ForgeRender
DefaultGroupName=ForgeRender
DisableProgramGroupPage=yes
OutputDir=..\..\dist
OutputBaseFilename=ForgeRender-{#AppVersion}-setup-x64
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
; Installazione per tutti gli utenti (richiede amministratore) o solo per l'utente corrente.
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
ChangesAssociations=yes
UninstallDisplayIcon={app}\forgerender.exe

[Languages]
Name: "italian"; MessagesFile: "compiler:Languages\Italian.isl"
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked
Name: "assoc"; Description: "Apri i file .frproject con ForgeRender"; GroupDescription: "Associazioni file:"

[Files]
Source: "{#BinDir}\forgerender.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#BinDir}\forgerender-cli.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#BinDir}\forgerender-mcp.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#BinDir}\forgerender-step.exe"; DestDir: "{app}"; Flags: ignoreversion skipifsourcedoesntexist
Source: "..\licenses\*"; DestDir: "{app}\licenze"; Flags: ignoreversion
; Open Image Denoise: CPU e GPU NVIDIA (CUDA). I moduli HIP/SYCL (AMD/Intel GPU) non sono inclusi.
Source: "{#OidnDir}\bin\OpenImageDenoise.dll"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#OidnDir}\bin\OpenImageDenoise_core.dll"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#OidnDir}\bin\OpenImageDenoise_device_cpu.dll"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#OidnDir}\bin\OpenImageDenoise_device_cuda.dll"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#OidnDir}\bin\tbb12.dll"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#OidnDir}\doc\LICENSE.txt"; DestDir: "{app}\licenze\OpenImageDenoise"; Flags: ignoreversion
Source: "{#OidnDir}\doc\third-party-programs*.txt"; DestDir: "{app}\licenze\OpenImageDenoise"; Flags: ignoreversion
Source: "LEGGIMI.txt"; DestDir: "{app}"; Flags: ignoreversion isreadme
Source: "..\..\lang\*.json"; DestDir: "{app}\lang"; Flags: ignoreversion
Source: "..\..\manuale\*"; DestDir: "{app}\manuale"; Excludes: "inventario.json,INVENTARIO.md"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "..\..\assets\materiali\*"; DestDir: "{app}\materiali"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "..\..\assets\anteprima\*"; DestDir: "{app}\anteprima"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{group}\ForgeRender"; Filename: "{app}\forgerender.exe"
Name: "{group}\ForgeRender (modalità sicura)"; Filename: "{app}\forgerender.exe"; Parameters: "--safe-mode"
Name: "{group}\{cm:UninstallProgram,ForgeRender}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\ForgeRender"; Filename: "{app}\forgerender.exe"; Tasks: desktopicon

[Registry]
Root: HKA; Subkey: "Software\Classes\.frproject"; ValueType: string; ValueName: ""; ValueData: "ForgeRender.Project"; Flags: uninsdeletevalue; Tasks: assoc
Root: HKA; Subkey: "Software\Classes\ForgeRender.Project"; ValueType: string; ValueName: ""; ValueData: "Progetto ForgeRender"; Flags: uninsdeletekey; Tasks: assoc
Root: HKA; Subkey: "Software\Classes\ForgeRender.Project\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\forgerender.exe,0"; Tasks: assoc
Root: HKA; Subkey: "Software\Classes\ForgeRender.Project\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\forgerender.exe"" ""%1"""; Tasks: assoc

[Run]
Filename: "{app}\forgerender.exe"; Description: "{cm:LaunchProgram,ForgeRender}"; Flags: nowait postinstall skipifsilent
