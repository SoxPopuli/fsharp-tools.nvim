local M = {}

local function split_path(path)
    local sep = nil
    if vim.fn.has("win32") == 1 then
        sep = "\\"
    else
        sep = "/"
    end

    return vim.split(path, sep, { trimempty = true })
end

local function round(num)
    return math.floor(num + 0.5)
end

local function create_fake_buffer(title, float, writefn)
    local bufnr = vim.api.nvim_create_buf(false, false)

    vim.bo[bufnr].buftype = 'nofile'
    vim.bo[bufnr].bufhidden = 'delete'
    vim.bo[bufnr].swapfile = false

    local winid = vim.api.nvim_get_current_win()
    local wininfo = vim.fn.getwininfo(winid)[1]

    local width = round(wininfo.width * 0.8)
    local height = round(wininfo.height * 0.8)

    local x = round(wininfo.width - width) / 2.0
    local y = round(wininfo.height - height) / 2.0

    if float then
        vim.api.nvim_open_win(bufnr, true, {
            relative = "win",
            width = width,
            height = height,
            col = x,
            row = y,
            border = "rounded",
            title = title,
        })
    else
        vim.cmd('vertical split')
        vim.api.nvim_win_set_buf(winid, bufnr)
    end
    vim.api.nvim_create_autocmd({ "BufUnload" }, {
        buffer = bufnr,
        callback = writefn
    })

    return bufnr
end

local function get_project_file_path()
    local bufnr = 0
    local endpoint = 'fsharp/workspacePeek'
    local args = { Directory = '.', Deep = 0, ExcludedDirs = {} }

    local res = vim.lsp.buf_request_sync(bufnr, endpoint, args)

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

local function get_files_from_project(file)
    local uv = vim.loop
    local fd = uv.fs_open(file, 'r', 0)

    local max_size = 1024 * 1024 * 1024
    local content = uv.fs_read(fd, max_size)

    local files = {}
    -- for line in content:gmatch([[%s+<Compile.*?Include *= *%"([^%"]+)%"]]) do
    for line in content:gmatch('<Compile.-Include -= -\"([^\"]+)\"') do
        table.insert(files, line)
    end

    uv.fs_close(fd)

    return files
end

-- Create Fake buffer for moving / editing / displaying file order in .fsproj files
-- Saving should cause a write to the relevant fsproj
function M.edit_file_order(opts)
    local o = opts or {}
    setmetatable(o, {__index={float = true}})

    local project = get_project_file_path()
    local parts = split_path(project)
    local project_name = parts[#parts]

    local bufnr = create_fake_buffer(project_name, o.float, function(args)

    end)
    local files = get_files_from_project(project)

    vim.api.nvim_buf_set_lines(bufnr, 0, #files, false, files)
end

function M.setup(opts)

end

return M
