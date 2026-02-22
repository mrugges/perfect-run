-- ConfigReader: Reads the overlay's config.json from the Script Extender IPC directory.

ConfigReader = {}

local IPC_VERSION = 1
local disabledStorylines = {}

--- Load config from disk.
function ConfigReader.Load()
    local content = Ext.IO.LoadFile("perfect-run/config.json", "user")
    if not content or content == "" then
        -- No config yet, that's fine
        disabledStorylines = {}
        return
    end

    local ok, parsed = pcall(Ext.Json.Parse, content)
    if not ok or type(parsed) ~= "table" then
        Ext.Utils.PrintWarning("[PerfectRun] Failed to parse config.json")
        return -- Keep previous config rather than resetting
    end

    -- Validate version
    local version = parsed.version or 0
    if version ~= IPC_VERSION then
        Ext.Utils.PrintWarning("[PerfectRun] Config version mismatch: expected " .. IPC_VERSION .. ", got " .. version)
        return -- Keep previous config
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
