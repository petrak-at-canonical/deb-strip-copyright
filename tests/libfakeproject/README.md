# libfakeproject

This directory is a fake debian project used for end-to-end testing.

During tests, the following folder structure is created in a temporary directory:

```
tempdir
|- libfakeproject-src.tar.xz
|  \- libfakeproject/
|     \- src/
|        |- main.py
|        \- illegal.py
\- libfakeproject/
   \- debian/
      |- changelog
      |- copyright
      \- ... etc ...
```

This roughly matches the common working directory setup.
