#!/bin/sh

set -e

red='\033[0;31m'
green='\033[0;32m'
yellow='\033[0;33m'
off='\033[0m'       

if [ "$1" != "release" ]; then
    echo -e "${red}Please let this be run by cargo release as a pre-release hook.${off}";
    exit 1
fi    


echo -e "${yellow}Running pre-release checks.${off}"

while true; do
    echo -n -e "${yellow}Did you update changelog.md (type 'yes' for yes)? ${off}"
    read answer
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

# Make sure tests work
if cargo test; then
    echo -e "${green}☑ All tests passed.${off}"
else
    echo -e "${red}☒ Fix test errors and try again.${off}"
    exit 1
fi

# Make sure docs can be generated
if cargo run docs  --schemas json-schemas/ --docs docs/generated/; then
    echo -e "${green}☑ Documentation generated.${off}"
else
    echo -e "${red}☒ Fix documentation errors and try again.${off}"
    exit 1
fi

# Make sure everything's committed
# (NOTE: This is a second check, since cargo release already does this, but 
# I've just generated docs, which will change this value. Naturally, if the docs
# did change something, the pre-flight will fail. However, that might indicate to
# the user they might have forgotten a change to log. But, next time they run
# this, it should succeed.)
# Make sure everything's committed
# (NOTE: This is a second check, since cargo release already does this, but 
# I've just generated docs, which will change this value. Naturally, if the docs
# did change something, the pre-flight will fail. However, that might indicate to
# the user they might have forgotten a change to log. But, next time they run
# this, it should succeed.)
if [ -n "$(git status --porcelain)" ]; then
    echo -e "${red}☒ There are uncommitted changes for git.${off}"
    exit 1
else
    echo -e "${green}☑ All changes committed.${off}"
fi

# Make sure remote and local are in sync
# NOTE: cargo release does not seem to check this, At best it checks if remote
# is ahead, not if it's behind.
# NOTE: cargo release does not seem to check this, At best it checks if remote
# is ahead, not if it's behind.
echo "Fetching from remote..."
git fetch
if [ "$(git rev-parse HEAD)" = "$(git rev-parse @{u})" ]; then
    echo -e "${green}☑ Local and remote are in sync.${off}"
else
    echo -e "${red}☒ Local and remote are not in sync.${off}"
    exit 1
fi  


echo -e "${green}Ready to Release${off}"

