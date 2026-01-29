# Shell Completion

Chant provides shell completion for bash, zsh, fish, and PowerShell. Tab completion helps you discover commands, flags, and options without memorizing them.

## Quick Setup

### Bash

```bash
# System-wide (requires root)
chant completion bash | sudo tee /etc/bash_completion.d/chant > /dev/null

# User-local
mkdir -p ~/.local/share/bash-completion/completions
chant completion bash > ~/.local/share/bash-completion/completions/chant
```

Then restart your shell or run:
```bash
source ~/.local/share/bash-completion/completions/chant
```

### Zsh

```bash
# If using Oh My Zsh
chant completion zsh > ~/.oh-my-zsh/completions/_chant

# Standard Zsh (ensure completions dir is in fpath)
mkdir -p ~/.zsh/completions
chant completion zsh > ~/.zsh/completions/_chant
```

Add to your `~/.zshrc` if the completions directory is not already in fpath:
```bash
fpath=(~/.zsh/completions $fpath)
autoload -Uz compinit && compinit
```

Then restart your shell or run:
```bash
source ~/.zshrc
```

### Fish

```bash
# Create completions directory if needed
mkdir -p ~/.config/fish/completions

# Generate completions
chant completion fish > ~/.config/fish/completions/chant.fish
```

Fish will automatically load the completions on next shell start.

### PowerShell

```powershell
# Add to your PowerShell profile
chant completion powershell >> $PROFILE

# Or create a separate file and source it
chant completion powershell > ~/.config/powershell/chant.ps1
```

Add to your `$PROFILE` if using a separate file:
```powershell
. ~/.config/powershell/chant.ps1
```

## What Gets Completed

Once installed, you can use tab completion for:

- **Commands**: `chant <TAB>` shows all available commands
- **Flags**: `chant work --<TAB>` shows available options for `work`
- **Subcommands**: `chant init <TAB>` shows subcommand options

## Verifying Installation

Test that completions are working:

```bash
# Type 'chant ' and press Tab
chant <TAB>

# Should show: add, approve, archive, cancel, cleanup, ...
```

## Updating Completions

After upgrading chant, regenerate your completions to pick up new commands:

```bash
# Example for bash
chant completion bash > ~/.local/share/bash-completion/completions/chant
```

## Troubleshooting

### Completions not loading

1. Verify the completion file exists:
   ```bash
   ls -la ~/.local/share/bash-completion/completions/chant  # bash
   ls -la ~/.zsh/completions/_chant                          # zsh
   ls -la ~/.config/fish/completions/chant.fish              # fish
   ```

2. Check your shell's completion system is enabled:
   - **Bash**: Ensure `bash-completion` package is installed
   - **Zsh**: Ensure `compinit` is called in `~/.zshrc`
   - **Fish**: Completions load automatically

3. Restart your shell completely (not just source the profile)

### Zsh: "compdef: unknown command or service"

Add this to your `~/.zshrc` before loading completions:
```bash
autoload -Uz compinit && compinit
```

### Bash: completion script errors

Ensure you have `bash-completion` installed:
```bash
# Debian/Ubuntu
sudo apt install bash-completion

# macOS (Homebrew)
brew install bash-completion@2
```
