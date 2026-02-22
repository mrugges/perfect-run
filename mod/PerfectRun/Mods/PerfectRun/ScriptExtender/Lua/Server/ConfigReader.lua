-- ConfigReader: Reads the overlay's config.json from the Script Extender IPC directory.

ConfigReader = {}

local disabledStorylines = {}

--- Get the path to the IPC config file.
local function GetConfigPath()
    local seDir = Ext.IO.GetPathOverride("ScriptExtender")
    if seDir then
        return seDir .. "/perfect-run/config.json"
    end
    -- Fallback: construct path manually
    local localAppData = os.getenv("LOCALAPPDATA")
    if localAppData then
        return localAppData .. "\\Larian Studios\\Baldur's Gate 3\\Script Extender\\perfect-run\\config.json"
    end
    return nil
end

--- Load config from disk.
function ConfigReader.Load()
    local path = GetConfigPath()
    if not path then
        Ext.Utils.PrintWarning("[PerfectRun] Could not determine config path")
        return
    end

    local content = Ext.IO.LoadFile("perfect-run/config.json", "user")
    if not content or content == "" then
        -- No config yet, that's fine
        disabledStorylines = {}
        return
    end

    local ok, parsed = pcall(Ext.Json.Parse, content)
    if not ok or type(parsed) ~= "table" then
        Ext.Utils.PrintWarning("[PerfectRun] Failed to parse config.json")
        disabledStorylines = {}
        return
    end

    if type(parsed.disabled_storylines) == "table" then
        disabledStorylines = {}
        for _, id in ipairs(parsed.disabled_storylines) do
            disabledStorylines[id] = true
        end
        Ext.Utils.Print("[PerfectRun] Config loaded: " .. #parsed.disabled_storylines .. " storylines disabled")
    else
        disabledStorylines = {}
    end
end

--- Get the set of currently disabled storyline IDs.
--- @return table<string, boolean> Map of storyline ID -> true if disabled
function ConfigReader.GetDisabledStorylines()
    return disabledStorylines
end

--- Check if a specific storyline is disabled.
--- @param id string Storyline ID
--- @return boolean
function ConfigReader.IsDisabled(id)
    return disabledStorylines[id] == true
end

return ConfigReader
