#!/bin/bash

# Define colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${BLUE}=======================================${NC}"
echo -e "${BLUE}   LLM Agent Installation Assistant    ${NC}"
echo -e "${BLUE}=======================================${NC}"

# --- Function: Interactive Menu (Arrow Keys) ---
function select_option {
    local options=("$@")
    local selected=0
    local key

    # Hide cursor
    tput civis

    while true; do
        # Print options
        for i in "${!options[@]}"; do
            if [ $i -eq $selected ]; then
                echo -e "${GREEN}> ${options[$i]}${NC}"
            else
                echo -e "  ${options[$i]}"
            fi
        done

        # Read key input
        read -rsn1 key

        # Handle arrow keys (which are multi-byte sequences)
        if [[ $key == $'\x1b' ]]; then
            read -rsn2 key
            if [[ $key == "[A" ]]; then # Up
                ((selected--))
                if [ $selected -lt 0 ]; then selected=$((${#options[@]} - 1)); fi
            elif [[ $key == "[B" ]]; then # Down
                ((selected++))
                if [ $selected -ge ${#options[@]} ]; then selected=0; fi
            fi
        elif [[ $key == "" ]]; then # Enter key
            break
        fi

        # Clear lines to redraw
        for i in "${!options[@]}"; do
            tput cuu1
            tput el
        done
    done

    # Restore cursor
    tput cnorm

    # Return selected item
    SELECTED_ITEM="${options[$selected]}"
}

# --- 1. Check Prerequisites ---
echo -e "\n${BLUE}[1/5] Checking prerequisites...${NC}"

# Rust
if ! command -v cargo &>/dev/null; then
    echo -e "${RED}Error: Rust (cargo) is not installed.${NC}"
    echo "Please install it from https://rustup.rs/"
    exit 1
fi
echo -e "${GREEN}✓ Rust found${NC}"

# Docker
if ! command -v docker &>/dev/null; then
    echo -e "${RED}Error: Docker is not installed.${NC}"
    exit 1
fi
if ! docker info &>/dev/null; then
    echo -e "${RED}Error: Docker daemon is not running.${NC}"
    exit 1
fi
echo -e "${GREEN}✓ Docker running${NC}"

# Ollama
if ! command -v ollama &>/dev/null; then
    echo -e "${RED}Error: Ollama is not installed.${NC}"
    echo "Please install from https://ollama.com/"
    exit 1
fi
echo -e "${GREEN}✓ Ollama found${NC}"

# --- 2. Check & Select Models ---
echo -e "\n${BLUE}[2/5] Checking Ollama Models...${NC}"

# Get list of models (skip header 'NAME')
MODELS=($(ollama list | tail -n +2 | awk '{print $1}'))

if [ ${#MODELS[@]} -eq 0 ]; then
    echo -e "${RED}No models found in Ollama.${NC}"
    echo "Downloading default model 'qwen2.5-coder'..."
    ollama pull qwen2.5-coder
    MODELS=("qwen2.5-coder:latest")
fi

echo "Select which model the Agent should use (Use Arrow Keys + Enter):"
select_option "${MODELS[@]}"
CHOSEN_MODEL=$SELECTED_ITEM

echo -e "Selected Model: ${GREEN}$CHOSEN_MODEL${NC}"

# --- 3. Configure Workspace ---
echo -e "\n${BLUE}[3/5] Configuring Workspace...${NC}"
echo "Where should the Agent store its files?"
read -e -p "Path (default: ./workspace): " USER_PATH
[ -z "$USER_PATH" ] && USER_PATH="./workspace"

# Convert to absolute path
mkdir -p "$USER_PATH"
FULL_PATH=$(realpath "$USER_PATH")
echo -e "Workspace: ${GREEN}$FULL_PATH${NC}"

# --- 4. Generate Config File ---
echo -e "\n${BLUE}[4/5] Saving Configuration...${NC}"

CONFIG_DIR="$HOME/.config/copilot_rust_llama"
CONFIG_FILE="$CONFIG_DIR/config.toml"

mkdir -p "$CONFIG_DIR"

cat >"$CONFIG_FILE" <<EOF
model = "$CHOSEN_MODEL"
workspace_path = "$FULL_PATH"
ollama_url = "http://localhost:11434/api/chat"
EOF

echo -e "Config saved to ${GREEN}$CONFIG_FILE${NC}"

# --- 5. Build Project ---
echo -e "\n${BLUE}[5/5] Building Rust Project (Release)...${NC}"
cargo build --release

if [ $? -ne 0 ]; then
    echo -e "${RED}Build failed.${NC}"
    exit 1
fi

# --- Create Runner ---
WRAPPER_NAME="run_agent.sh"
BINARY_PATH="./target/release/copilot_rust_llama"

cat >$WRAPPER_NAME <<EOF
#!/bin/bash
# The Rust app now reads from ~/.config/copilot_rust_llama/config.toml automatically.
# You can also manually override via ENV vars if needed, but it's not required.

"$BINARY_PATH"
EOF

chmod +x $WRAPPER_NAME

echo -e "\n${GREEN}=======================================${NC}"
echo -e "${GREEN}   Installation Complete!              ${NC}"
echo -e "${GREEN}=======================================${NC}"
echo ""
echo "Configuration is stored in: $CONFIG_FILE"
echo "To start the agent, run:"
echo -e "${BLUE}./$WRAPPER_NAME${NC}"
echo ""
