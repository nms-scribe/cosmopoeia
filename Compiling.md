# Linux

Unfortunately, I did not need to install everything from scratch for this as I had all prerequisites installed already. Here is my guess:

1) Install [`rustup`](https://www.rust-lang.org/tools/install) and run the script. This takes some time but is fairly easy.
2) Make sure you have gdal library installed on your machine, it should be available in your package manager.
3) Make sure you have [libclang installed](https://rust-lang.github.io/rust-bindgen/requirements.html), to use the bindgen feature in gdal. This feature is necessary in case your current gdal version is higher than that supported by the gdal crate. For arch linux, this is done with the command `yay -S clang`.
3) Run `cargo build` from this directory.



# Windows

Getting this compiling on Windows was more difficult, There are more steps and several *very large* extras that have to be downloaded. The gdal install was discovered through piecing information together from various sources and trial and error, I suspect because most people who develop this already have most of this installed. There may be easier ways to do this, if so, please let me know. This is my first attempt at Windows development in at least ten years, and going through this sort of process is one of the reasons.

1) You will need to install gdal on your system, as described under the installation section of [Readme.md](/Readme.md).
2) Install [`rustup`](https://www.rust-lang.org/tools/install) and run the script. It may ask you to download and install an enormous (about 1.5 GB) Visual Studio package from Microsoft to complete this task.
3) The above does not install the `gdal_i.lib` file needed by the rust library. So it's necessary to install the gdal SDK:
    a) Download `gdal` SDK from [GISInternals Development Kits](https://gisinternals.com/sdk.php).  I chose the latest one for `MSVC 2022` and `x64`: `release-1930-x64-dev`. The number after release will depend on which one you download, as well.
    b) unzip that into a `windows` folder under this project so that it's contents are in `cosmopoeia\windows\release-1930-x64-dev`, where `cosmopoeia` is the root of this project.
4) The build routine needs to find the actual library installed by gdal using a tool called `pkg-config`. 
    a) download [`pkg-config`](https://sourceforge.net/projects/pkgconfiglite/).
    b) unzip that into the `windows` folder as well, so that it's contents are in `cosmopoeia\windows\pkg-config-lite-0.28-1_bin-win32`. There will be a single folder inside that called `pkg-config-lite-0.28-1`
5) Set some environment variables: *NOTE: The deploy_windows.ers script sets them in this manner*
    a) So pkg-config can be found: `set PATH=%CD%\windows\pkg-config-lite-0.28-1_bin-win32\pkg-config-lite-0.28-1\bin;%PATH%`
    b) So `gdal_i.lib` can be found: `set GDAL_HOME=%CD%\windows\release-1930-x64-dev\release-1930-x64`
    c) So `gdal.dll` can be found: `set PKG_CONFIG_PATH=%homedrive%%homepath%\miniconda3\Library\lib\pkgconfig`
6) `cargo build` should now run successfully in the same session where the environment variables were set.

# Macintosh

I don't have access to Mac machines for development. So, if anyone wants to support compiling and running this product on Macintosh products, please let me know.