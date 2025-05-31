#!/usr/bin/env pwsh

# Configuration
$RUST_SRC_DIR = "rust/src"
$GDEXTENSION_FILE = "godot/Project.gdextension"
$ICON_FILENAME = "RustNode.svg"

# Check if rust source directory exists
if (-not (Test-Path -Path $RUST_SRC_DIR -PathType Container)) {
    Write-Error "Directory '$RUST_SRC_DIR' does not exist"
    exit 1
}

# Check if .gdextension file exists
if (-not (Test-Path -Path $GDEXTENSION_FILE -PathType Leaf)) {
    Write-Error "File '$GDEXTENSION_FILE' does not exist"
    exit 1
}

Write-Host "Searching for GodotClass structs in $RUST_SRC_DIR..."

# Find all Rust files and search for structs with #[derive(...GodotClass...)]
$struct_names = @()

$rust_files = Get-ChildItem -Path $RUST_SRC_DIR -Filter "*.rs" -Recurse -File

foreach ($file in $rust_files) {
    Write-Host "Checking file: $($file.FullName)"
    
    $lines = Get-Content -Path $file.FullName
    $found_godot_class = $false
    
    for ($i = 0; $i -lt $lines.Count; $i++) {
        $line = $lines[$i]
        
        # Check if current line contains a derive macro with GodotClass
        if ($line -match '^\s*#\[derive\(.*GodotClass.*\)\]') {
            $found_godot_class = $true
            continue
        }
        
        # If we found GodotClass derive, keep looking for more derive macros or the struct
        if ($found_godot_class) {
            # Check if this is another derive macro (continue looking)
            if ($line -match '^\s*#\[derive\(.*\)\]') {
                continue
            }
            
            # Check if this is any other attribute (skip it)
            if ($line -match '^\s*#\[') {
                continue
            }
            
            # Check if this is the struct declaration
            if ($line -match '^\s*(pub\s+)?struct\s+([A-Za-z_][A-Za-z0-9_]*)') {
                $struct_name = $matches[2]
                $struct_names += $struct_name
                Write-Host "Found struct: $struct_name in $($file.FullName)"
                $found_godot_class = $false
                continue
            }
            
            # If we hit any other non-empty, non-comment line, reset
            if ($line -match '^\s*$' -or $line -match '^\s*//') {
                # Empty line or comment, continue looking
                continue
            } else {
                # Hit some other code, reset the flag
                $found_godot_class = $false
            }
        }
    }
}

if ($struct_names.Count -eq 0) {
    Write-Host "No structs implementing GodotClass found."
    exit 0
}

Write-Host "Found $($struct_names.Count) struct(s) implementing GodotClass:"
$struct_names | ForEach-Object { Write-Host $_ }

# Read the gdextension file
$gdextension_content = Get-Content -Path $GDEXTENSION_FILE

# Check if [icons] section exists
$icons_section_exists = $gdextension_content | Select-String -Pattern '^\[icons\]' -Quiet

if ($icons_section_exists) {
    Write-Host "[icons] section found, updating..."
    
    $new_content = @()
    $in_icons_section = $false
    $icons_section_processed = $false
    
    foreach ($line in $gdextension_content) {
        if ($line -match '^\[icons\]') {
            $new_content += $line
            $in_icons_section = $true
            $icons_section_processed = $true
            
            # Add all struct entries after [icons] header
            foreach ($struct_name in $struct_names) {
                $new_content += "$struct_name = `"res://assets/$ICON_FILENAME`""
            }
            continue
        }
        
        # Check if we're entering a new section
        if ($line -match '^\[.*\]' -and $in_icons_section) {
            $in_icons_section = $false
        }
        
        # Skip existing icon entries in [icons] section
        if ($in_icons_section -and $line -match '^[A-Za-z_][A-Za-z0-9_]*\s*=') {
            continue
        }
        
        $new_content += $line
    }
    
    # Write the updated content back to the file
    $new_content | Set-Content -Path $GDEXTENSION_FILE
    
} else {
    Write-Host "[icons] section not found, adding at the end..."
    
    # Add [icons] section at the end of file
    $new_content = $gdextension_content + @("", "[icons]")
    
    foreach ($struct_name in $struct_names) {
        $new_content += "$struct_name = `"res://assets/$ICON_FILENAME`""
    }
    
    $new_content | Set-Content -Path $GDEXTENSION_FILE
}

Write-Host "Successfully updated $GDEXTENSION_FILE"
Write-Host "Added icons for: $($struct_names -join ', ')"
Write-Host ""
Write-Host "Changes made:"
foreach ($struct_name in $struct_names) {
    Write-Host "  $struct_name = `"res://assets/$ICON_FILENAME`""
}