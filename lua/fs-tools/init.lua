local M = {}

local util = require('fs-tools.utils')

M.buffer_data = {}

local sep = nil
if vim.fn.has("win32") == 1 then
    sep = "\\"
else
    sep = "/"
end

local function split_path(path)
    return vim.split(path, sep, { trimempty = false })
end

local function join_path(root, part)
    return root .. sep .. part
end

local function round(num)
    return math.floor(num + 0.5)
end

local function open_file(path, mode)
    local fd = vim.loop.fs_open(path, mode, 0)
    return fd
end

local function read_file_to_end(file)
    local max_size = 1024 * 1024 * 1024
    local content = vim.loop.fs_read(file, max_size)
    return content
end

local function close_file(file)
    vim.loop.fs_close(file)
end

local function write_buffer_to_project(bufnr)
    local path = M.buffer_data[bufnr].path
    local file = open_file(path, 'r')
    local content = read_file_to_end(file)

    local lines = vim.api.nvim_buf_get_lines(bufnr, 0, -1, false)

    local function line_iter(indent)
        local first = indent .. '<ItemGroup>\n'
        local last = indent .. '</ItemGroup>'

        local items = ""

        for _, value in pairs(lines) do
            local trimmed = vim.trim(value)
            if trimmed:len() == 0 then
                goto continue
            end

            local next =
                indent ..
                indent ..
                '<Compile Include="' ..
                trimmed ..
                '" />\n'
            items = items .. next

            ::continue::
        end

        return first .. items .. last
    end

    --local pattern = '(%s-<Compile.-Include%s-=%s-\")[^\"]+(\"%s-/>)'
    local pattern = '([ \\t]+)<ItemGroup>.-</ItemGroup>'
    local edited = content:gsub(pattern, line_iter)

    close_file(file)

    file = open_file(path, 'w')
    vim.loop.fs_write(file, edited)
    print(path .. ' saved')
    close_file(file)
end

local function setup_autocommands(bufnr)
    vim.api.nvim_create_autocmd({ "BufUnload" }, {
        buffer = bufnr,
        callback = function(args)
            M.buffer_data[args.buf] = nil
        end
    })

    vim.api.nvim_create_autocmd({ 'BufWriteCmd' }, {
        buffer = bufnr,
        callback = function(args)
            write_buffer_to_project(args.buf)
            vim.bo[args.buf].modified = false
        end
    })
end

local function create_fake_buffer(title, float)
    local bufnr = vim.api.nvim_create_buf(false, false)

    vim.api.nvim_buf_set_name(bufnr, title)

    vim.bo[bufnr].buftype = 'acwrite'
    vim.bo[bufnr].bufhidden = 'delete'
    vim.bo[bufnr].swapfile = false

    local size = vim.api.nvim_list_uis()[1]
    local max_width = size.width
    local max_height = size.height

    local width = max_width * 0.8
    local height = max_height * 0.8

    local x = (max_width - width) / 2.0
    local y = (max_height - height) / 2.0

    if float then
        vim.api.nvim_open_win(bufnr, true, {
            relative = "editor",
            width = round(width),
            height = round(height),
            col = x,
            row = y / 1.2,
            border = "rounded",
            title = title,
        })
    else
        vim.cmd('vertical split')
        vim.api.nvim_win_set_buf(0, bufnr)
        vim.cmd('vertical resize 35')
    end

    setup_autocommands(bufnr)

    return bufnr
end

local function get_project_file_path()
    local bufnr = 0
    local endpoint = 'fsharp/workspacePeek'
    local args = { Directory = '.', Deep = 0, ExcludedDirs = {} }

    local res = vim.lsp.buf_request_sync(bufnr, endpoint, args)

    if not res then
        return nil
    end

    local index = 0
    while res[index] == nil do
        index = index + 1

        if index == 100 then
            error("project not found")
        end
    end

    local content = res[index].result.content
    return vim.json.decode(content).Data.Found[1].Data.Fsprojs[1]
end

local function get_files_from_project(path)
    local file = open_file(path, 'r')
    local content = read_file_to_end(file)

    local files = {}
    for line in content:gmatch('<Compile.-Include%s-=%s-\"([^\"]+)\"') do
        table.insert(files, line)
    end

    close_file(file)
    return files
end

local function clear_undos(bufnr)
    local old_levels = vim.bo[bufnr].undolevels
    vim.bo[bufnr].undolevels = -1
    vim.cmd.execute [["normal a \<BS>"]]
    vim.bo[bufnr].undolevels = old_levels
    vim.bo[bufnr].modified = false
end

local function set_keybinds(bufnr)
    local keymap = vim.keymap.set

    keymap('n', 'q', '<cmd>:q<cr>', { nowait = true, buffer = bufnr })

    keymap('n', '<CR>', function()
        local line = vim.api.nvim_get_current_line()
        local path_root = M.buffer_data[bufnr].root
        local path = join_path(path_root, line)

        if M.buffer_data[bufnr].float then
            vim.api.nvim_win_close(0, false)
            vim.cmd.e(path)
        else
            local tabpage = vim.api.nvim_get_current_tabpage()
            local windows = vim.api.nvim_tabpage_list_wins(tabpage)

            local buffers = util.map(windows, function(x)
                return {
                    win = x,
                    buf = vim.api.nvim_win_get_buf(x)
                }
            end)
            buffers = util.filter(buffers, function(x) return bufnr ~= x.buf end)
            local names = util.map(buffers, function(x)
                return {
                    win = x.win,
                    name = vim.api.nvim_buf_get_name(x.buf)
                }
            end)

            if #names == 1 then
                vim.api.nvim_set_current_win(util.first(names).win)
                vim.cmd.e(path)
            else
                vim.ui.select(names, {
                    prompt = 'Choose buffer to replace: ',
                    format_item = function(item)
                        local msg = item.name
                        return msg
                    end
                }, function(choice)
                    vim.api.nvim_set_current_win(choice.win)
                    vim.cmd.e(path)
                end)
            end
        end
    end, { nowait = true, buffer = bufnr })
end

local function get_root_from_parts(parts)
    local path = parts[1]
    for i = 2, #parts - 1, 1 do
        path = join_path(path, parts[i])
    end

    return path
end

local function setup_buffer(bufnr, files, project_path_parts, project_path, is_float)
    vim.api.nvim_buf_set_lines(bufnr, 0, #files, false, files)
    clear_undos(bufnr)
    set_keybinds(bufnr)
    M.buffer_data[bufnr] = {
        root = get_root_from_parts(project_path_parts),
        path = project_path,
        float = is_float,
    }
end

-- Create Fake buffer for moving / editing / displaying file order in .fsproj files
-- Saving should cause a write to the relevant fsproj
function M.edit_file_order(opts)
    local o = opts or {}
    setmetatable(o, { __index = { float = true } })

    local project = get_project_file_path()
    if not project then
        print('No project found for current file')
        return
    end


    local parts = split_path(project)
    local project_name = parts[#parts]

    local bufnr = create_fake_buffer(project_name, o.float)
    local files = get_files_from_project(project)

    setup_buffer(bufnr, files, parts, project, o.float)
end

function M.setup(opts)

end

return M
