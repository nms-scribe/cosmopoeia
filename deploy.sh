#!/bin/sh

set -e

red='\033[0;31m'
green='\033[0;32m'
yellow='\033[0;33m'
off='\033[0m'
bold='\033[1m'


# need to know whether this is a minor or patch

while true; do
    echo -n -e "${yellow}What version level are you bumping? [major (x.0.0) | minor (x.y.0) | ${bold}patch (x.y.z)${off}${yellow} | rc (x.y.z-rc.M)] ${off}"
    read level
    case $level in
        major ) echo -e "${green}Bumping major version level${off}"; break;;
        minor ) echo -e "${green}Bumping minor version level${off}"; break;;
        patch ) echo -e "${green}Bumping patch version level${off}"; break;;
        rc ) echo -e "${green}Bumping release candidate version level${off}"; break;;
        * ) echo -e "${red}Please entery \"major\", \"minor\", \"patch\", or \"rc\".${off}";;
    esac
done

# Let's do some pre-checks

echo -e "${yellow}Running pre-release checks.${off}"

while true; do
    echo -n -e "${yellow}Did you update changelog.md (type 'yes' for yes)? ${off}"
    read answer
    case $answer in
        [Yy][Ee][Ss] ) echo -e "${green}☑ User claims that changelog.md is up to date.${off}"; break;;
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


echo -e "${green}☑ Ready to Release${off}"



# Are we going to execute?

while true; do
    echo -n -e "${yellow}Are you ready to do this (type '--execute' if you are, return if you aren't)? ${off}"
    read execarg
    case $execarg in
        '--execute' ) echo -e "${green}Okay, let's go.${off}"; break;;
        '' ) echo -e "${green}Okay, this will be a dry run only.${off}"; break;; 
        * ) echo -e "${red}Enter the argument or blank${off}";;
    esac
done

if cargo release $level $execarg; then
    echo -e "${green}☑ Version bumped.${off}"
else
    echo -e "${red}☒ Version could not be bumped, something's wrong.${off}"
    exit 1
fi

if cargo aur; then
    echo -e "${green}☑ Arch PKGBUILD and tarball generated.${off}"
else
    echo -e "${red}☒ PKGBUILD could not be generated, something's wrong.${off}"
    exit 1
fi

if [ "$level" != "rc" ]; then 
   if cargo release rc $execarg; then
        echo -e "${green}☑ Next release candidate generated for further development.${off}"
    else
        echo -e "${red}☒ Version could not be bumped, something's wrong.${off}"
        exit 1
    fi
fi

echo -e "${green}It's all done. Don't forget to upload the release to github with the correct version tag.${off}"
