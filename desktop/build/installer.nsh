# The two things the NSIS installer cannot be told, said in its own language.
#
# **This is the include electron-builder's `nsis` target reads** — named by
# `include` in `electron-builder.yml`, and found in `build/` because that is the
# configuration's `buildResources`. Its template offers a handful of macros to
# define (`templates/nsis/` inside app-builder-lib: `preInit`, `customInstall`,
# `customUnInstall` and the rest), and inserts each at the point its name says.
# Nothing else here runs.
#
# **The install directory**, which a configuration key cannot name for this
# installer: a per-user one-click install is put under the package's name,
# `verkstead-desktop`, and the product name is only used for an assisted or a
# per-machine one. `…\Programs\Verkstead` is what the msi used, what Launch on
# Startup's Run value names `Verkstead.exe` inside and what a machine's `PATH`
# entry already says, so it does not move.
#
# **And the `PATH` entry**, which is half of what a Windows download is for:
# `verkstead ask`, `verkstead guide` and the rest have to work in a terminal
# opened after the install. It names `resources\cli` inside the install rather
# than the install root — the root holds the launcher, `Verkstead.exe`, and
# Windows looks a name up without regard to case, so a root on `PATH` would
# answer `verkstead guide` with a window instead of the Guide (ADR-0020).
#
# Every macro is LogicLib and registers rather than labels and variables: a
# macro is inserted once in the installer and once in the uninstaller, a label
# may only be said once, and electron-builder compiles with warnings as errors,
# which an unused `Var` is.

!include LogicLib.nsh
!include WinMessages.nsh

# **The retired msi's UpgradeCode**, which is the one identifier every Verkstead
# msi was installed under — carried over from the Rust msi and never changed.
# Nothing reads it yet: it is what a machine still holding the msi is found by.
!define MSI_UPGRADE_CODE "{A4727089-F5B9-40A6-A196-1856E8D6827A}"

# What the install directory is called, under `%LOCALAPPDATA%\Programs`.
!define INSTALL_DIRECTORY_NAME "Verkstead"

# Where the sidecar sits inside the install — `extraResources` in
# `electron-builder.yml`, and the directory `src/cli.ts` looks in.
!define CLI_DIRECTORY "resources\cli"

# The directory, said where the template reads it. `setInstallModePerUser` in
# `multiUser.nsh` installs to `InstallLocation` under the install registry key
# when there is one — which is how a reinstall finds the install it replaces —
# and to the package-named default when there is not. So a first install is
# given the value before that macro reads it, and a later one keeps whatever
# the install before it recorded: written only when nothing is there, so an
# install somebody moved with `/D=` stays where they moved it.
#
# Not in the uninstaller's build pass, which runs this same `.onInit` on the
# machine packing the app.
!macro preInit
  !ifndef BUILD_UNINSTALLER
    ReadRegStr $0 HKCU "${INSTALL_REGISTRY_KEY}" InstallLocation
    ${If} $0 == ""
      WriteRegExpandStr HKCU "${INSTALL_REGISTRY_KEY}" InstallLocation \
        "$LOCALAPPDATA\Programs\${INSTALL_DIRECTORY_NAME}"
    ${EndIf}
  !endif
!macroend

# This user's `PATH`, in `$R0`, and whether it could be read, in `$R9`.
#
# **A `PATH` this cannot hold whole is left alone**, because what is read here
# is written back: NSIS strings have a fixed length, and a value longer than
# that reads as empty with the error flag set — the same answer as no value at
# all. Written back, that is somebody's whole `PATH` replaced with one entry. So
# an error is told apart from an absence by looking for the value by name, and a
# string that reached the length limit is taken to have been cut off.
!macro readUserPath
  StrCpy $R9 "readable"
  ClearErrors
  ReadRegStr $R0 HKCU "Environment" "Path"
  ${If} ${Errors}
    StrCpy $R0 ""
    StrCpy $R8 0
    ${Do}
      ClearErrors
      EnumRegValue $R7 HKCU "Environment" $R8
      ${If} ${Errors}
      ${OrIf} $R7 == ""
        ${Break}
      ${EndIf}
      ${If} $R7 == "Path"
        StrCpy $R9 "unreadable"
        ${Break}
      ${EndIf}
      IntOp $R8 $R8 + 1
    ${Loop}
  ${Else}
    StrLen $R8 $R0
    IntOp $R7 ${NSIS_MAX_STRLEN} - 1
    ${If} $R8 >= $R7
      StrCpy $R9 "unreadable"
    ${EndIf}
  ${EndIf}
!macroend

# `$R0` with every entry naming this install's CLI directory taken out, in
# `$R1`. Compared the way Windows compares a path — `StrCmp` takes no notice of
# case — and with a trailing separator off, so the msi's spelling of the same
# entry is the same entry. Empty entries go too: an entry taken out of the
# middle would otherwise leave `;;` where it was.
!macro withoutCli
  StrCpy $R1 ""
  StrCpy $R2 $R0
  ${DoUntil} $R2 == ""
    StrLen $R4 $R2
    StrCpy $R3 0
    ${DoWhile} $R3 < $R4
      StrCpy $R5 $R2 1 $R3
      ${If} $R5 == ";"
        ${Break}
      ${EndIf}
      IntOp $R3 $R3 + 1
    ${Loop}
    StrCpy $R5 $R2 $R3
    IntOp $R3 $R3 + 1
    StrCpy $R2 $R2 "" $R3

    StrCpy $R6 $R5
    StrCpy $R7 $R6 1 -1
    ${If} $R7 == "\"
      StrCpy $R6 $R6 -1
    ${EndIf}

    ${If} $R5 == ""
    ${OrIf} $R6 == "$INSTDIR\${CLI_DIRECTORY}"
      # Not kept.
    ${ElseIf} $R1 == ""
      StrCpy $R1 $R5
    ${Else}
      StrCpy $R1 "$R1;$R5"
    ${EndIf}
  ${Loop}
!macroend

# Tell every open window the environment changed, so that a terminal started
# from Explorer afterwards is handed the new `PATH` rather than the one Explorer
# started with. Bounded: a window that does not answer is not waited on.
!macro announceEnvironment
  SendMessage ${HWND_BROADCAST} ${WM_SETTINGCHANGE} 0 "STR:Environment" /TIMEOUT=5000
!macroend

!macro saveRegisters
  Push $R0
  Push $R1
  Push $R2
  Push $R3
  Push $R4
  Push $R5
  Push $R6
  Push $R7
  Push $R8
  Push $R9
!macroend

!macro restoreRegisters
  Pop $R9
  Pop $R8
  Pop $R7
  Pop $R6
  Pop $R5
  Pop $R4
  Pop $R3
  Pop $R2
  Pop $R1
  Pop $R0
!macroend

# **On install, the entry goes on the end, once.** Any copy already there is
# taken out first, so a reinstall over this install — or an update, which is a
# reinstall — leaves one entry rather than one more. In `HKCU`, because a
# per-user install has no business in the machine's list, and as an expandable
# string, which is the type Windows gives this user's `Path` itself.
!macro customInstall
  !insertmacro saveRegisters
  !insertmacro readUserPath
  ${If} $R9 == "readable"
    !insertmacro withoutCli
    ${If} $R1 == ""
      StrCpy $R1 "$INSTDIR\${CLI_DIRECTORY}"
    ${Else}
      StrCpy $R1 "$R1;$INSTDIR\${CLI_DIRECTORY}"
    ${EndIf}
    WriteRegExpandStr HKCU "Environment" "Path" $R1
    !insertmacro announceEnvironment
  ${Else}
    DetailPrint "This user's PATH is too long to change safely, so $INSTDIR\${CLI_DIRECTORY} was not added to it."
  ${EndIf}
  !insertmacro restoreRegisters
!macroend

# **And on uninstall it comes off**, with everything else on this user's
# `PATH` left as it was. A `PATH` that held nothing but this entry is taken
# away rather than left empty, the way it was before there was an install.
!macro customUnInstall
  !insertmacro saveRegisters
  !insertmacro readUserPath
  ${If} $R9 == "readable"
  ${AndIf} $R0 != ""
    !insertmacro withoutCli
    ${If} $R1 != $R0
      ${If} $R1 == ""
        DeleteRegValue HKCU "Environment" "Path"
      ${Else}
        WriteRegExpandStr HKCU "Environment" "Path" $R1
      ${EndIf}
      !insertmacro announceEnvironment
    ${EndIf}
  ${EndIf}
  !insertmacro restoreRegisters
!macroend
