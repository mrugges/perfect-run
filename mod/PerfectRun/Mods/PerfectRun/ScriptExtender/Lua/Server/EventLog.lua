-- EventLog: Tracks blocked events and writes status.json for the overlay to read.

EventLog = {}

local IPC_VERSION = 1
local blockedEvents = {}
local isActive = false
local MAX_EVENTS = 100

--- Set whether the mod is active.
function EventLog.SetActive(active)
    isActive = active
end

--- Log a blocked event.
--- @param storylineId string Which storyline triggered the block
--- @param description string What was blocked
function EventLog.LogBlocked(storylineId, description)
    table.insert(blockedEvents, {
        storyline_id = storylineId,
        description = description,
        timestamp = os.time()
    })

    -- Keep the log bounded
    if #blockedEvents > MAX_EVENTS then
        table.remove(blockedEvents, 1)
    end

    Ext.Utils.Print("[PerfectRun] BLOCKED: [" .. storylineId .. "] " .. description)
end

--- Write the status file for the overlay.
function EventLog.WriteStatus()
    local status = {
        version = IPC_VERSION,
        active = isActive,
        last_update = os.time(),
        blocked_events = blockedEvents
    }

    local json = Ext.Json.Stringify(status)
    Ext.IO.SaveFile("perfect-run/status.json", json)
end

return EventLog
