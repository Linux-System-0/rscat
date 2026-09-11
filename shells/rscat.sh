# rscat: rainbow cat — https://github.com/anomalyco/rscat
# Added by `rscat --init sh`. Idempotent, safe to re-source.
case ":$PATH:" in
  *":$HOME/.local/bin:"*) ;;
  *) export PATH="$HOME/.local/bin:$PATH" ;;
esac
