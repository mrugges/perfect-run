@echo off
setlocal enabledelayedexpansion

:: =============================================================================
:: unpack-bg3.bat — Unpack BG3 game files for storyline modding
::
:: Extracts Gustav.pak + Shared.pak, converts flags/dialogs to readable XML,
:: and decompiles story.div.osi — all fully automated.
::
:: Usage:
::   unpack-bg3.bat                                    (auto-detect everything)
::   unpack-bg3.bat --lslib "C:\path\to\lslib"
::   unpack-bg3.bat --bg3 "D:\Games\Baldurs Gate 3"
:: =============================================================================

set "SCRIPT_DIR=%~dp0"
set "OUTPUT=%SCRIPT_DIR%bg3-unpacked"
set "LSLIB_DIR="
set "BG3_DIR="

:: ---- Parse arguments ----
:parse_args
if "%~1"=="" goto :done_args
if /i "%~1"=="--lslib" ( set "LSLIB_DIR=%~2" & shift & shift & goto :parse_args )
if /i "%~1"=="--bg3"   ( set "BG3_DIR=%~2"   & shift & shift & goto :parse_args )
if /i "%~1"=="--output" ( set "OUTPUT=%~2"    & shift & shift & goto :parse_args )
echo Unknown argument: %~1
exit /b 1
:done_args

:: ---- Find LSLib ----
if defined LSLIB_DIR goto :find_tools

:: Check if already extracted next to this script
if exist "%SCRIPT_DIR%lslib" (
    set "LSLIB_DIR=%SCRIPT_DIR%lslib"
    goto :find_tools
)

:: Auto-extract from Downloads
set "LSLIB_ZIP="
for %%Z in ("%USERPROFILE%\Downloads\ExportTool-*.zip") do set "LSLIB_ZIP=%%~Z"

if defined LSLIB_ZIP if exist "!LSLIB_ZIP!" (
    echo Found LSLib archive: !LSLIB_ZIP!
    echo Extracting to %SCRIPT_DIR%lslib ...
    powershell -Command "Expand-Archive -LiteralPath '!LSLIB_ZIP!' -DestinationPath '%SCRIPT_DIR%lslib' -Force"
    if errorlevel 1 ( echo ERROR: Failed to extract. & exit /b 1 )
    set "LSLIB_DIR=%SCRIPT_DIR%lslib"
    goto :find_tools
)

echo ERROR: Could not find LSLib.
echo Download from: https://github.com/Norbyte/lslib/releases
echo Then run:  unpack-bg3.bat --lslib "C:\path\to\extracted\folder"
exit /b 1

:: ---- Find Divine.exe and StoryDecompiler.exe ----
:find_tools
set "DIVINE="
set "STORY_DECOMPILER="

:: Search recursively for the tools
for /r "%LSLIB_DIR%" %%F in (Divine.exe) do (
    if /i "%%~nxF"=="Divine.exe" (
        set "DIVINE=%%F"
    )
)
for /r "%LSLIB_DIR%" %%F in (StoryDecompiler.exe) do (
    if /i "%%~nxF"=="StoryDecompiler.exe" (
        set "STORY_DECOMPILER=%%F"
    )
)

if not defined DIVINE (
    echo ERROR: Divine.exe not found in %LSLIB_DIR%
    exit /b 1
)
echo Divine.exe:          %DIVINE%
if defined STORY_DECOMPILER (
    echo StoryDecompiler.exe: %STORY_DECOMPILER%
) else (
    echo StoryDecompiler.exe: not found (story decompilation will be skipped)
)

:: ---- Find BG3 installation ----
if defined BG3_DIR goto :check_bg3

set "_T=C:\Program Files (x86)\Steam\steamapps\common\Baldurs Gate 3"
if exist "!_T!\Data\Gustav.pak" ( set "BG3_DIR=!_T!" & goto :check_bg3 )
set "_T=C:\Program Files\Steam\steamapps\common\Baldurs Gate 3"
if exist "!_T!\Data\Gustav.pak" ( set "BG3_DIR=!_T!" & goto :check_bg3 )
set "_T=D:\SteamLibrary\steamapps\common\Baldurs Gate 3"
if exist "!_T!\Data\Gustav.pak" ( set "BG3_DIR=!_T!" & goto :check_bg3 )
set "_T=D:\Steam\steamapps\common\Baldurs Gate 3"
if exist "!_T!\Data\Gustav.pak" ( set "BG3_DIR=!_T!" & goto :check_bg3 )
set "_T=E:\SteamLibrary\steamapps\common\Baldurs Gate 3"
if exist "!_T!\Data\Gustav.pak" ( set "BG3_DIR=!_T!" & goto :check_bg3 )

echo ERROR: Could not find BG3 installation.
echo Specify manually:  unpack-bg3.bat --bg3 "D:\Games\Baldurs Gate 3"
exit /b 1

:check_bg3
set "DATA_DIR=!BG3_DIR!\Data"
if not exist "!DATA_DIR!\Gustav.pak" (
    echo ERROR: Gustav.pak not found in !DATA_DIR!
    exit /b 1
)
echo BG3 data:            !DATA_DIR!
echo Output:              %OUTPUT%
echo.

:: ---- Create output directory ----
if not exist "%OUTPUT%" mkdir "%OUTPUT%"

:: ---- Step 1: Extract Gustav.pak ----
echo === Step 1/5: Extracting Gustav.pak ===
if exist "%OUTPUT%\Gustav" (
    echo   Skipping, already extracted. Delete bg3-unpacked\Gustav to redo.
) else (
    echo   This is large, may take a few minutes...
    call :run_divine extract-package "!DATA_DIR!\Gustav.pak" "%OUTPUT%\Gustav"
)
echo.

:: ---- Step 2: Extract Shared.pak ----
echo === Step 2/5: Extracting Shared.pak ===
if exist "%OUTPUT%\Shared" (
    echo   Skipping, already extracted.
) else (
    call :run_divine extract-package "!DATA_DIR!\Shared.pak" "%OUTPUT%\Shared"
)
echo.

:: ---- Step 3: Convert flags to readable XML ----
echo === Step 3/5: Converting flag files (.lsf to .lsx) ===
call :convert_flags "Shared\Public\Shared\Flags"    "Flags\Shared"
call :convert_flags "Shared\Public\SharedDev\Flags"  "Flags\SharedDev"
call :convert_flags "Gustav\Public\Gustav\Flags"     "Flags\Gustav"
call :convert_flags "Gustav\Public\GustavDev\Flags"  "Flags\GustavDev"
echo.

:: ---- Step 4: Convert dialogs to readable XML ----
echo === Step 4/5: Converting dialog files (.lsf to .lsx) ===
set "DLG_SRC=%OUTPUT%\Gustav\Mods\GustavDev\Story\DialogsBinary"
set "DLG_DST=%OUTPUT%\converted\Dialogs"
if exist "%DLG_SRC%" (
    if exist "%DLG_DST%" (
        echo   Skipping, already converted.
    ) else (
        echo   Converting dialogs, this takes a while...
        "!DIVINE!" -a convert-resources -g bg3 -s "%DLG_SRC%" -d "%DLG_DST%" -i lsf -o lsx -l error
        echo   Done.
    )
) else (
    echo   DialogsBinary not found, skipping.
)
echo.

:: ---- Step 5: Decompile story.div.osi ----
echo === Step 5/5: Decompiling story.div.osi ===
set "STORY_FILE=%OUTPUT%\Gustav\Mods\GustavDev\Story\story.div.osi"
set "GOALS_DIR=%OUTPUT%\goals"

if not exist "%STORY_FILE%" (
    echo   story.div.osi not found in extracted Gustav.pak, skipping.
    goto :summary
)

if exist "%GOALS_DIR%" (
    echo   Skipping, already decompiled.
    goto :summary
)

if defined STORY_DECOMPILER (
    echo   Decompiling with StoryDecompiler.exe...
    if not exist "%GOALS_DIR%" mkdir "%GOALS_DIR%"
    "!STORY_DECOMPILER!" --input "%STORY_FILE%" --output "%GOALS_DIR%"
    if errorlevel 1 (
        echo   WARNING: StoryDecompiler failed. Try ConverterApp.exe manually.
    ) else (
        echo   Done.
    )
) else (
    echo   StoryDecompiler.exe not found. Use ConverterApp.exe manually:
    echo     1. Open ConverterApp.exe
    echo     2. Go to "Story (OSI) tools" tab
    echo     3. Story file:  %STORY_FILE%
    echo     4. Goal output: %GOALS_DIR%
    echo     5. Click "Decompile"
)
echo.

:: ---- Summary ----
:summary
echo =========================================================================
echo.
echo   Done! Files are in: %OUTPUT%
echo.
echo   Search with perfect-run CLI:
echo.
echo     build.bat run -p bg3-cli -- search-flags guardian --dir "%OUTPUT%\converted"
echo     build.bat run -p bg3-cli -- search-dialogs dream --dir "%OUTPUT%\converted"
echo     build.bat run -p bg3-cli -- search-goals emperor --dir "%OUTPUT%\goals"
echo.
echo   Layout:
echo     converted\Flags\     Flag definitions as readable XML
echo     converted\Dialogs\   Dialog files as readable XML
echo     goals\               Decompiled Osiris goal scripts
echo     Gustav\              Raw extracted Gustav.pak
echo     Shared\              Raw extracted Shared.pak
echo.
echo =========================================================================

endlocal
exit /b 0

:: ---- Helper: convert flags if source exists ----
:convert_flags
set "_SRC=%OUTPUT%\%~1"
set "_DST=%OUTPUT%\converted\%~2"
if exist "!_SRC!" (
    if exist "!_DST!" (
        echo   %~2: skipping, already converted.
    ) else (
        echo   %~2: converting...
        "!DIVINE!" -a convert-resources -g bg3 -s "!_SRC!" -d "!_DST!" -i lsf -o lsx -l error
    )
) else (
    echo   %~2: source not found, skipping.
)
exit /b 0

:: ---- Helper: run Divine.exe for pak extraction ----
:run_divine
"!DIVINE!" -a %~1 -g bg3 -s "%~2" -d "%~3" -l error
if errorlevel 1 (
    echo   ERROR: Failed.
    exit /b 1
)
echo   Done.
exit /b 0
