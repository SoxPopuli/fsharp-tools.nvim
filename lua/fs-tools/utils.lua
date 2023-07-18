local M = {}

---@param iter table
---@param fn function
---@return table
M.filter = function (iter, fn)
    local t = {}
    for _, v in pairs(iter) do
        if fn(v) == true then
            table.insert(t, v)
        end
    end
    return t
end

---@param iter table
---@param fn function
---@return table
M.map = function (iter, fn)
    local t = {}
    for _, v in pairs(iter) do
        table.insert(t, fn(v))
    end
    return t
end

---Returns the first item in the table
---@param iter table
---@return any|nil
M.first = function(iter)
    for _, value in pairs(iter) do
        return value
    end
    return nil
end


return M

