# fsharp-tools.nvim
Assorted tools for working with F#

Requires cargo

```lua
require('fsharp-tools')
```

### Example Lazy Config
```lua
{
  'SoxPopuli/fsharp-tools.nvim',
  ft = { 'fsharp', 'xml' },
  build = './build.sh -r',
  opts = {
    indent = 2, --project file indent per tag
    max_depth = 4, --maximum level of upwards directory searches
  },
},

```

### Functions

| function | parameters | return | description |
| --- | --- | --- | --- |
| `edit_file_order` | `floating: bool` |  | Opens a temporary buffer that lists the files included in the fsproj file in order.<br>Writing to the buffer will change the project file to match the content of the buffer.<br>Pressing enter on a line will take you to the relevant file|
