local M = {}

local util = require('fsharp-tools.utils')

---@class Core
---@field find_fsproj fun(file_path: string, max_depth: integer): string
---@field get_files_from_project fun(file_path: string): string[]
---@field write_files_to_project fun(file_path: string, files: string[], indent: integer?)
---@field get_file_name fun(file_name: string): string
---@field get_cumulative_path_parts fun(path: string): string
local core = require('fsharp_tools_rs')

---@class BufferData
---@field project_name string
---@field project_path string
---@field project_root string
---@field float boolean

---@class Settings
---@field indent integer
---@field max_depth integer
local settings = {
  indent = 2,
  max_depth = 4,
}

local sep = nil
if vim.fn.has('win32') == 1 then
  sep = '\\'
else
  sep = '/'
end

---@param path string
---@return string[]
local function split_path(path)
  return vim.split(path, sep, { trimempty = false })
end

---@param s string
---@return string
local function last_char(s)
  if s:len() == 0 then
    return ''
  else
    local last_index = s:len()
    return s:sub(last_index, last_index)
  end
end

---@param root string
---@param part string
---@return string
local function join_path(root, part)
  if last_char(root) == part then
    return root .. part
  else
    return root .. sep .. part
  end
end

---@param parts string[]
---@return string
local function get_root_from_parts(parts)
  local path = parts[1]
  for i = 2, #parts - 1, 1 do
    path = join_path(path, parts[i])
  end

  return path
end

---@param num number
---@return integer
local function round(num)
  return math.floor(num + 0.5)
end

---@param bufnr integer
---@param data BufferData
local function write_buffer_to_project(bufnr, data)
  local lines = vim.api.nvim_buf_get_lines(bufnr, 0, -1, false)
  core.write_files_to_project(data.project_path, lines, settings.indent)
end

---@param bufnr integer
---@param data BufferData
local function setup_autocommands(bufnr, data)
  --vim.api.nvim_create_autocmd({ 'BufUnload' }, {
  --  buffer = bufnr,
  --  callback = function(args)
  --    M.buffer_data[args.buf] = nil
  --  end,
  --})

  vim.api.nvim_create_autocmd({ 'BufWriteCmd' }, {
    buffer = bufnr,
    callback = function(args)
      write_buffer_to_project(args.buf, data)
      vim.bo[args.buf].modified = false
    end,
  })
end

---@param title string
---@param data BufferData
---@return integer bufnr
local function create_fake_buffer(title, data)
  local bufnr = vim.api.nvim_create_buf(false, false)

  vim.api.nvim_buf_set_name(bufnr, 'fs-tools:' .. title)

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

  if data.float then
    vim.api.nvim_open_win(bufnr, true, {
      relative = 'editor',
      width = round(width),
      height = round(height),
      col = x,
      row = y / 1.2,
      border = 'rounded',
      title = title,
    })
  else
    vim.cmd('vertical split')
    vim.api.nvim_win_set_buf(0, bufnr)
    vim.cmd('vertical resize 35')
  end

  setup_autocommands(bufnr, data)

  return bufnr
end

---@param bufnr integer
local function clear_undos(bufnr)
  local old_levels = vim.bo[bufnr].undolevels
  vim.bo[bufnr].undolevels = -1
  vim.cmd.execute([["normal a \<BS>"]])
  vim.bo[bufnr].undolevels = old_levels
  vim.bo[bufnr].modified = false
end

---@param bufnr integer
---@param data BufferData
local function set_keybinds(bufnr, data)
  local keymap = vim.keymap.set

  keymap('n', 'q', '<cmd>:q<cr>', { nowait = true, buffer = bufnr })

  keymap('n', '<CR>', function()
    local line = vim.api.nvim_get_current_line()
    local path_root = data.project_root
    local path = join_path(path_root, line)
    path = path .. '.fs'           -- re-add extension

    path = path:gsub('[\\/]', sep) -- escape backslashes

    if data.float then
      vim.api.nvim_win_close(0, false)
      vim.cmd.e(path)
    else
      local tabpage = vim.api.nvim_get_current_tabpage()
      local windows = vim.api.nvim_tabpage_list_wins(tabpage)

      local buffers = util.map(windows, function(x)
        return {
          win = x,
          buf = vim.api.nvim_win_get_buf(x),
        }
      end)
      buffers = util.filter(buffers, function(x)
        return bufnr ~= x.buf
      end)
      local names = util.map(buffers, function(x)
        return {
          win = x.win,
          name = vim.api.nvim_buf_get_name(x.buf),
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
          end,
        }, function(choice)
          vim.api.nvim_set_current_win(choice.win)
          vim.cmd.e(path)
        end)
      end
    end
  end, { nowait = true, buffer = bufnr })
end

---@param bufnr integer
---@param files string[]
---@param data BufferData
local function setup_buffer(bufnr, files, data)
  vim.api.nvim_buf_set_lines(bufnr, 0, #files, false, files)
  clear_undos(bufnr)
  set_keybinds(bufnr, data)
end

---if x is nil, return default else return x
---@generic T
---@param x T | nil
---@param default T
---@return T
local function or_default(x, default)
  if x == nil then
    return default
  else
    return x
  end
end

-- Create Fake buffer for moving / editing / displaying file order in .fsproj files
-- Saving should cause a write to the relevant fsproj
---@param is_floating boolean | nil
function M.edit_file_order(is_floating)
  local floating = or_default(is_floating, false)

  local file = vim.fn.bufname() --[[@type string]]
  local project = core.find_fsproj(file, settings.max_depth)

  local project_name = core.get_file_name(project)

  local buffer_data = {
    project_name = project_name,
    project_path = project,
    project_root = get_root_from_parts(split_path(project)),
    float = is_floating,
  }

  local bufnr = create_fake_buffer(project_name, buffer_data)
  local files = core.get_files_from_project(project)

  setup_buffer(bufnr, files, buffer_data)
end

function M.insert_module_name()
  ---@type string
  local file_path = vim.fn.expand('%:r.')

  ---@class Parts
  ---@field count integer
  ---@field item string
  local parts = (function()
    local file_parts = util.string_split(file_path, sep)
    local parts = {}
    for i = 1, #file_parts do
      local ends = util.take_back(file_parts, i)
      parts[i] = {
        count = i,
        item = util.join(ends, '.'),
      }
    end
    return parts
  end)()

  vim.ui.select(
    parts,
    {
      ---@param line Parts
      format_item = function(line)
        return line.count .. '. ' .. line.item
      end,
    },
    ---@param choice Parts|nil
    function(choice)
      if choice == nil then
        return
      end
      ---@type string
      local current_line = vim.api.nvim_get_current_line()
      local is_line_empty = current_line:len() == 0 or current_line:gsub('\\w+', ''):len() == 0

      local text_to_insert = 'module ' .. choice.item

      local line_nr = vim.fn.line('.')
      if is_line_empty then
        vim.api.nvim_set_current_line(text_to_insert)
      else
        vim.api.nvim_buf_set_lines(0, line_nr, line_nr, false, { text_to_insert })
      end
    end
  )
end

---@param opts Settings
function M.setup(opts)
  settings = vim.tbl_extend('force', settings, opts)
end

return M
