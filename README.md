# ex-post-progress

Gives you a progress bar for an existing program that's processing a file descriptor.
Usage:
```
$ ex-post-progress <pid> <file-paths>
```

It will find the file descriptors which are `file-paths` opened by `pid` automatically, and draw you a progress bar, exiting once the file decriptors have reached the end of the file.

You can also leave `<file-paths>` blank, in which case it will track all open file descriptors which point to regular files.
