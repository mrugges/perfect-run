-- StorylineBlocker: Core blocking logic.
-- Registers Osiris listeners to intercept and block storyline events
-- based on the current config from the overlay.

StorylineBlocker = {}

-- Registry of storyline blockers. Each entry maps a storyline ID to a table of:
--   hooks: list of { type, register_fn, unregister_fn }
-- Blockers are registered/unregistered dynamically as config changes.

local registeredBlockers = {}  -- storyline_id -> { active = bool, listeners = {} }

-- ============================================================================
-- Blocking primitives
-- ============================================================================

--- Block a flag by clearing it whenever it gets set.
--- @param storylineId string
--- @param flagGuid string The flag GUID to block
--- @param description string
local function RegisterFlagBlocker(storylineId, flagGuid, description)
    -- Listen for global flag set
    local handle1 = Ext.Osiris.RegisterListener("GlobalFlagSet", 1, "after", function(flag)
        if flag == flagGuid and ConfigReader.IsDisabled(storylineId) then
            Osi.GlobalClearFlag(flagGuid)
            EventLog.LogBlocked(storylineId, "Cleared global flag: " .. description)
        end
    end)

    -- Listen for object flag set (character-specific flags)
    local handle2 = Ext.Osiris.RegisterListener("FlagSet", 3, "after", function(flag, speaker, dialogInstance)
        if flag == flagGuid and ConfigReader.IsDisabled(storylineId) then
            Osi.ClearFlag(flagGuid, speaker)
            EventLog.LogBlocked(storylineId, "Cleared flag on " .. tostring(speaker) .. ": " .. description)
        end
    end)

    return { handle1, handle2 }
end

--- Block a dialog by stopping it when it starts.
--- @param storylineId string
--- @param dialogPattern string Pattern to match against dialog resource name
--- @param description string
local function RegisterDialogBlocker(storylineId, dialogPattern, description)
    local handle = Ext.Osiris.RegisterListener("DialogStarted", 2, "after", function(dialog, dialogInstance)
        if string.find(dialog, dialogPattern) and ConfigReader.IsDisabled(storylineId) then
            Osi.DialogRequestStopForDialog(dialog, dialogInstance)
            EventLog.LogBlocked(storylineId, "Stopped dialog: " .. description .. " (" .. dialog .. ")")
        end
    end)

    return { handle }
end

--- Block a quest by closing it when it updates.
--- @param storylineId string
--- @param questId string
--- @param description string
local function RegisterQuestBlocker(storylineId, questId, description)
    local handle = Ext.Osiris.RegisterListener("QuestUpdateUnlocked", 3, "after", function(quest, subquest, character)
        if quest == questId and ConfigReader.IsDisabled(storylineId) then
            Osi.QuestClose(questId)
            EventLog.LogBlocked(storylineId, "Closed quest: " .. description .. " (" .. quest .. ")")
        end
    end)

    return { handle }
end

--- Clear a flag when a specific event occurs.
--- @param storylineId string
--- @param eventName string Osiris event to listen for
--- @param flagGuid string Flag to clear
--- @param description string
local function RegisterClearFlagOnEvent(storylineId, eventName, flagGuid, description)
    -- This is a generic listener - the event name and arity might vary
    -- For now, support common events with known arities
    local handle = Ext.Osiris.RegisterListener("GlobalFlagSet", 1, "after", function(flag)
        if flag == eventName and ConfigReader.IsDisabled(storylineId) then
            Osi.GlobalClearFlag(flagGuid)
            EventLog.LogBlocked(storylineId, "Cleared flag on event: " .. description)
        end
    end)

    return { handle }
end

-- ============================================================================
-- Storyline definitions (loaded at startup)
-- ============================================================================

-- Hard-coded storyline hooks. These match the storylines.toml definitions.
-- In the future, these could be loaded from a TOML/JSON file.

local STORYLINE_HOOKS = {
    guardian_emperor = {
        -- Guardian/Emperor dream sequences
        -- Flag GUIDs need to be discovered from unpacked game files
        -- Placeholder entries for now - replace with actual GUIDs
        flags = {
            -- { guid = "ACTUAL_GUID_HERE", description = "Guardian dream trigger" },
        },
        dialogs = {
            -- { pattern = "YOURURL_GuardianDream", description = "Guardian dream dialog" },
        },
        quests = {},
    },
    dark_urge = {
        flags = {},
        dialogs = {},
        quests = {},
    },
    -- Additional storylines can be added here
}

-- ============================================================================
-- Public API
-- ============================================================================

--- Apply config changes. Called when the config is reloaded.
--- @param disabledStorylines table<string, boolean>
function StorylineBlocker.ApplyConfig(disabledStorylines)
    -- Register blockers for newly disabled storylines
    for storylineId, hooks in pairs(STORYLINE_HOOKS) do
        if disabledStorylines[storylineId] then
            if not registeredBlockers[storylineId] then
                registeredBlockers[storylineId] = { active = true, listeners = {} }

                -- Register flag blockers
                for _, flagHook in ipairs(hooks.flags or {}) do
                    local handles = RegisterFlagBlocker(storylineId, flagHook.guid, flagHook.description)
                    for _, h in ipairs(handles) do
                        table.insert(registeredBlockers[storylineId].listeners, h)
                    end
                end

                -- Register dialog blockers
                for _, dialogHook in ipairs(hooks.dialogs or {}) do
                    local handles = RegisterDialogBlocker(storylineId, dialogHook.pattern, dialogHook.description)
                    for _, h in ipairs(handles) do
                        table.insert(registeredBlockers[storylineId].listeners, h)
                    end
                end

                -- Register quest blockers
                for _, questHook in ipairs(hooks.quests or {}) do
                    local handles = RegisterQuestBlocker(storylineId, questHook.quest_id, questHook.description)
                    for _, h in ipairs(handles) do
                        table.insert(registeredBlockers[storylineId].listeners, h)
                    end
                end

                Ext.Utils.Print("[PerfectRun] Registered blockers for: " .. storylineId)
            end
        end
    end

    -- Note: Osiris listeners cannot be unregistered once registered in BG3SE.
    -- The check inside each listener callback (ConfigReader.IsDisabled) handles
    -- dynamic enable/disable without needing to deregister.
end

return StorylineBlocker
