-- Wireshark dissector for j143/6g simulated protocol traces.
-- Load with: wireshark -X lua_script:wireshark/6g_trace.lua

local sixg_trace = Proto("sixg_trace", "6G Simulated Protocol Trace")
local fields = {
    magic = ProtoField.bytes("sixg_trace.magic", "Magic"),
    version = ProtoField.uint16("sixg_trace.version", "Format version", base.DEC),
    json_length = ProtoField.uint32("sixg_trace.json_length", "Event JSON length", base.DEC),
    event_json = ProtoField.string("sixg_trace.event_json", "Event JSON"),
}
sixg_trace.fields = fields

function sixg_trace.dissector(buffer, pinfo, tree)
    local length = buffer:len()
    if length < 12 then return 0 end
    if buffer(0, 4):bytes():tohex() ~= "36475452" then return 0 end

    pinfo.cols.protocol = "6G-TRACE"
    local subtree = tree:add(sixg_trace, buffer(), "6G Simulated Protocol Trace")
    subtree:add(fields.magic, buffer(0, 4))
    subtree:add(fields.version, buffer(4, 2))
    subtree:add(fields.json_length, buffer(8, 4))

    local json_length = buffer(8, 4):uint()
    if length >= 12 + json_length then
        subtree:add(fields.event_json, buffer(12, json_length):string())
        pinfo.cols.info = "Simulated 6G protocol event"
    end
    return length
end

DissectorTable.get("ethertype"):add(0x88B6, sixg_trace)
