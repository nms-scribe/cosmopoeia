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

while true; do
    echo -n -e "${yellow}Are you ready to do this (type '--execute' if you are, return if you aren't)? ${off}"
    read execarg
    case $execarg in
        '--execute' ) echo -e "${green}Okay, let's go.${off}"; break;;
        '' ) echo -e "${green}Okay, this will be a dry run only.${off}"; break;; 
        * ) echo -e "${red}Enter the argument or blank${off}";;
    esac
done

cargo release $level $execarg

cargo aur

if [ "$level" != "rc" ]; then 
   cargo release rc $execarg
fi


