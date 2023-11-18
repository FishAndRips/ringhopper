# Ringhopper

Ringhopper is a pure-Rust library used for creating and manipulating tag data for Halo: Combat Evolved. It also contains
the Invader toolset which directly interfaces with this library.

## Layout

- `src` contains the main source code
    - `invader` - This is where the command-line shell code is located. This is to provide a high quality command-line
    interface for users.
    - `ringhopper` - This is where the main, high level tag processing code is located. This contains the following:
        - Processing code for creating assets such as new tags from data
        - Calculation for physics, etc.
    - `ringhopper-definitions` - This is where low-level structure processing code exists. This contains the following:
        - Tag groups
        - Definitions for tag structures
        - Definitions for primitives such as vectors, colors, planes, etc. used to make up a tag
        - Processing code for manipulating primitives
        - Processing code for parsing structures

## License

Invader and Ringhopper are licensed under version 3 of the GNU General Public License as published by the Free Software
Foundation. It is not licensed under any other version of this license.

