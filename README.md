# fsharp-tools.nvim
Assorted tools for working with F#

Requires `fsautocomplete` or `ionide` LSPs to be running

### Functions
Currently contains one function
```lua
    require('fs-tools').edit_file_order({ float = true })
    -- or
    require('fs-tools').edit_file_order({ float = false })
```

Calling `edit_file_order` creates a temporary buffer that is a representation of the currently opened fs file.
Editing the buffer and writing to it will be reflected in the relevant .fsproj file.

Also pressing enter on an item will navigate to the file.
