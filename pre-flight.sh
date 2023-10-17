#/bin/sh

# Regular Colors
red='\033[0;31m'          # Red
green='\033[0;32m'        # Green
off='\033[0m'       # Text Reset



while true; do
    read -p "Did you update changelog.md (type 'yes' for yes)? " answer
    case $answer in
        [Yy][Ee][Ss] ) echo -e "${green}☑ User claims that changelong.md is up to date.${off}"; break;;
        * ) echo -e "${red}☒ Please update changelog.md and try again.${off}"; exit 1;
    esac
done

# The -D turns all warnings into errors, ensuring we get an exit code of 1 for this script.
if cargo clippy -- -D warnings; then
    echo -e "${green}☑ Clippy is happy.${off}"
else
    echo -e "${red}☒ Fix clippy warnings and try again.${off}"
    exit 1
fi

if cargo test; then
    echo -e "${green}☑ All tests passed.${off}"
else
    echo -e "${red}☒ Fix test errors and try again.${off}"
    exit 1
fi

if cargo run docs  --schemas json-schemas/ --docs docs/generated/; then
    echo -e "${green}☑ Documentation generated.${off}"
else
    echo -e "${red}☒ Fix documentation errors and try again.${off}"
    exit 1
fi

if [ -n "$(git status --porcelain)" ]; then
    echo -e "${red}☒ There are uncommitted changes for git.${off}"
    exit 1
else
    echo -e "${green}☑ All changes committed.${off}"
fi


echo "Ready to Release"