local M = {}

---@param iter table
---@param fn function
---@return table
function M.filter(iter, fn)
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
function M.map(iter, fn)
    local t = {}
    for _, v in pairs(iter) do
        table.insert(t, fn(v))
    end
    return t
end

---Returns the first item in the table
---@param iter table
---@return any|nil
function M.first(iter)
    for _, value in pairs(iter) do
        return value
    end
    return nil
end

---@param s string
---@param delim string
---@return string[]
function M.string_split(s, delim)
    local parts = {}
    local part_count = 1
    for i = 1, s:len() do
        local char = s:sub(i, i)
        if parts[part_count] == nil then
            table.insert(parts, part_count, char)
        elseif char == delim then
            part_count = part_count + 1
        else
            parts[part_count] = parts[part_count] .. char
        end
    end

    return parts
end

---@generic T
---@param array T[]
---@param count integer
---@return T[]
function M.take(array, count)
    local output = {}
    for i = 1, count do
        output[i] = array[i]
    end

    return output
end

---@generic T
---@param array T[]
---@param count integer
---@return T[]
function M.take_back(array, count)
    local output = {}
    for i = #array - count + 1, #array, 1 do
        table.insert(output, array[i])
    end
    return output
end

---@generic T
---@param array T[]
---@return T[]
function M.reverse(array)
    local output = {}
    for i = #array, 1, -1 do
        output[i] = array[#array - i + 1]
    end

    return output
end

---@param array string[]
---@param delim string
---@return string
function M.join(array, delim)
    if #array == 0 then
        return ''
    end

    local output = array[1]
    for i = 2, #array do
        output = output .. delim .. array[i]
    end

    return output
end

return M
