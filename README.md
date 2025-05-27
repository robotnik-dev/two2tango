# Setup

## On Github
Set the following repository variable and secrets

### vars:
- GODOT_VERSION
- ITCHIO_GAMENAME (for "web" version)
- TARGETS: list of strings ("linux" is mandatory)
  >["linux", "windows", "mac", "web"] 

### secrets:
- PAT
- BUTLER_API_KEY (for "web" version)
- ITCHIO_USERNAME (for "web" version)
- (Optional) DICORD_WEBHOOK
- (Optional) ITCHIO_SECRET_URL

## Locally
- pull the repository
- install rust via rustup
- install VS build tools
  - MSVC vXXX ... build tools
  - Windows 11 SDK
- build the project
  > When using VSCode, there are already convient tasks predefined to build the game

## New Godot version
When switching to a new godot version follow these steps
- `compatibility_minimum = 4.x` in Project.gdextension
- set new `GODOT_VERSION` within Github
