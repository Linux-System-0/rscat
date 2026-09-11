# rscat: rainbow cat — https://github.com/anomalyco/rscat
# Added by `rscat --init fish`. Idempotent, safe to re-source.
if not contains -- $HOME/.local/bin $PATH
    set -gx PATH $HOME/.local/bin $PATH
end
