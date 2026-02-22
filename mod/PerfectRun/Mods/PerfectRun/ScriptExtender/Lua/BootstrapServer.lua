-- PerfectRun BG3SE Mod - Server Bootstrap
-- Reads config from the overlay and blocks storyline events accordingly.

Ext.Require("Server/ConfigReader.lua")
Ext.Require("Server/EventLog.lua")
Ext.Require("Server/StorylineBlocker.lua")

local POLL_INTERVAL_MS = 2000

local function Initialize()
    Ext.Utils.Print("[PerfectRun] Mod loaded. Waiting for gameplay start...")

    Ext.Osiris.RegisterListener("LevelGameplayStarted", 2, "after", function(level, isEditorMode)
        Ext.Utils.Print("[PerfectRun] Gameplay started on level: " .. level)
        EventLog.SetActive(true)

        -- Initial config load
        ConfigReader.Load()
        StorylineBlocker.ApplyConfig(ConfigReader.GetDisabledStorylines())

        -- Start polling timer
        Ext.Timer.WaitFor(POLL_INTERVAL_MS, function()
            ConfigReader.Load()
            StorylineBlocker.ApplyConfig(ConfigReader.GetDisabledStorylines())
            EventLog.WriteStatus()
        end, POLL_INTERVAL_MS)
    end)
end

Initialize()
