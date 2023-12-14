#!/bin/bash

# Call this script as `./wasm-inpect.sh`
set -euo pipefail

wat_file="./query_engine.wat"

# Check if the .wat file exists
if [ ! -f "$wat_file" ]; then
    echo "Error: $wat_file not found."
    exit 1
fi

USE_LINK=false
for arg in "$@"; do
    if [ "$arg" == "--link" ]; then
        USE_LINK=true
        break
    fi
done

# Inspect the specified function name in the .wat file.
# Use as `func <function_name> [--link]`.
inspect_function() {
    local function_name="$1"

    # Use sed to find the line number where the function starts
    local -r start_line=$(sed -n "/(func \$$function_name/=" "$wat_file" | head -n 1)

    # Check if the function exists in the file
    if [ -z "$start_line" ]; then
        echo "Error: Function '$function_name' not found in '$wat_file'."
        exit 1
    fi

    __display_wat
}

# Inspect the specified data entry in the .wat file.
# Use as `data <data_entry> [--link]`.
inspect_data() {
    local data_entry="$1"

    # Use sed to find the line number where the data entry starts
    # (data (;139;)
    local -r start_line=$(sed -n "/(data (;$data_entry;)/=" "$wat_file" | head -n 1)

    # Check if the function exists in the file
    if [ -z "$start_line" ]; then
        echo "Error: Data entry '$data_entry' not found in '$wat_file'."
        exit 1
    fi

    __display_wat
}

__display_wat() {
    if [ "$USE_LINK" == true ]; then
        # --link option is used, so print the link to the function location
        local -r file_path=$(realpath "$wat_file")
        local vscode_link="$file_path:$start_line"
        echo -e "$vscode_link"
    else 
      # print the function definition, and allow the user to scroll through it
      less -N "+${start_line}" "$wat_file"
    fi
}

# Function to display help message
display_help() {

    local HELP_MESSAGE="
Usage: $0 <command> <function_name> [--link]
Inspect the specified function in the .wat file.

Commands:
  func <name>       Inspect the specified function
  data <entry>      Inspect the specified data entry

Options:
  --link            Show a link to the wat code rather than printing
                    the definition to stdout
  --help            Display this help message
"
    echo -e "$HELP_MESSAGE"
}

# Parse the command and execute the corresponding action
case "$1" in
    func)
        shift
        inspect_function "$@"
        ;;
    data)
        shift
        inspect_data "$@"
        ;;
    --help)
        display_help
        ;;
    *)
        echo "Unknown command: $1"
        exit 1
        ;;
esac

exit 0
