# 🤖 `md-command-check` 🤖

`md-command-check` is a simple CLI tool which allows you to test the code block your define in your
Markdown files. No more broken tutorials or instructions, use `md-command-check` to validate your 
tutorials and `README`s as if they were being run by an end user!

> [!WARNING]
> `md-command-check` is currently in its very early stages and I _might_ updated it in the future
> with additional (breaking) functionality.

## Installation

You can install `md-command-check` by running the following command:

```bash
cargo install --locked --git https://github.com/Trantorian1/md-command-check
```

## Usage

`md-command-check` allows you to specify the expected behavior of your code blocks, extract data
from `stdout` and `stderr` output and more! It achieves this through user-defined _directives_, 
which are short instructions specified inside of _html comments_, such as:

```md
<!-- ignore -->
```

> [!TIP]
> `md-command-check` will only run code inside of `bash` or `sh` command blocks.

Run `md-command-check` on this file to see it in action!

<!-- ignore -->

```bash
md-command-check --debug ./README.md
```

`md-command-check` will resolve each file in the order in which they are passed as arguments. You
can also list the code blocks to be executed in a file by running:

<!-- ignore -->

```bash
md-command-check --list --debug ./README.md
```

### `extract`

The `extract` directive can be used to retrieve data from a code block's `stdout` or `stderr` using
[regex captures].

```md
<!-- extract VAR_NAME "your_pattern (your_capture)"-->
```

You can then reference `<VAR_NAME>` throughout the following code blocks. For example, we can 
capture the output of `echo`:

<!-- extract MESSAGE "([\w\s]+)" -->

```bash
echo Hello World
```

And use it in another code block:

```bash
echo <MESSAGE>
```

> [!TIP]
> If you are viewing this `README` as a rendered page (as is the case by default on sites like 
> github) you will not be seeing the HTML comments and the directives inside of them. This is by 
> design, as it allows you to specify the expected behavior of your code blocks without each 
> directive visible to end users. If you wish to see the directives, switch to an un-rendered view 
> of this file or clone this repository locally.

### `env`

The `env` directive allows you to extract variables from the running environment:

```md
<!-- env VAR_NAME ENV_VAR -->
```

You can then reference `<VAR_NAME>` throughout the following code blocks. For example, you can 
capture the current working directory with `PWD`:

<!-- env YOUR_CURRENT_DIRECTORY PWD -->

```bash
echo <YOUR_CURRENT_DIRECTORY>
```

This is especially useful when certain commands in your markdown files require secrets.

<!-- ignore -->

> [!TIP]
> Variables declared with `extract` and `env` can be re-used across documents. If you run

> ```bash
> md-command-check --debug FILE1.md FILE2.md
> ```
>
> then variables defined in `FILE1` will be available when executing the code blocks in `FILE2`. 
> This allows you to chain instructions between multiple files, where the instructions in a file
> might be dependent on some setup you describe in another file.

### `alias`

The `alias` directive allows you to create new variables which reference the value of a variable
previously set by `extract` or `env`.

```md
<!-- alias MESSAGE MESSAGE_IN_FILE_1 -->
```

You can then reference `<MESSAGE_IN_FILE_1>` throughout the following code blocks as if it was
`<MESSAGE>`. This can be especially useful when inheriting variables from another file, and you want
to make this explicit to end users.

### `kill`

The `kill` directive tells `md-command-check` to forcefully shutdown the next code block once a
specified pattern has occurred on `stdout`.

```md
<!-- kill "YOUR_PATTERN" -->
```

> [!NOTE]
> This is due to a [limitation] in the way in which `md-command-check` works which does not allow it
> to handle blocking commands properly. This is something I plan to further develop, but since the
> changes required for this to work are substantial it has not been implemented as of now.

For example, the following will exit the code block before the call to `sleep`:

<!-- kill "exit now" -->

```bash
echo "exit now"
sleep infinity
```

> [!CAUTION]
> As part of the aforementioned [limitations], a new `bash` process will be spawned when killing the
> execution of a code block. _This causes all execution context to be lost, such as in-shell 
> environment changes, as well as resetting the current working directory._

### `ignore`

The `ignore` tells `md-command-check` that the following code block should _not_ be executed.

```md
<!-- ignore -->
```

This is useful when you are using `bash` or `sh` code blocks to showcase the output of certain 
commands without them containing any code which can actually be run, or when showcasing commands
which will explicitly fail. For example, the following code block will not be run:

<!-- ignore -->

```bash
exit 1
```

### `teardown`

The `teardown` directive specifies code to be run once `md-command-check` has finished executing a
file.

```md
<!-- teardown "cd .." -->
```

You can use this to reset the state of your execution environment in cases where you are running
`md-command-check` against multiple files. For example, if you run `cd my_crate` as part of `FILE1`,
you might want to setup a `teardown "cd .."` so that `FILE2` starts its execution from the root of
the repository.

> [!NOTE]
> At the moment there aren't any specific checks being enforced around `teardown`, meaning it will
> just be run as soon as it is encountered. This will change in the future.

### `file`

The `file` directive tells `md-command-check` to create a file out of the next code block.

```md
<!-- file path/to/file -->
```

> [!IMPORTANT]
> Keep in mind that file paths declared this way are _relative to the `.md` file in which they are 
> defined_!

You can even use variables in your code block and `md-command-check` will substitute in the correct
values! For example, the following block combine a `env` and `file` directive to create a `.env` 
with the name of the current user at the time of execution:

<!-- file .env -->
<!-- env YOU USER -->

```env
USER=<YOU>
```

Check out the new file:

```bash
cat .env
```

## Limitations

[regex capture]: https://www.regular-expressions.info/brackets.html
[limitiation]: #limitations
[limitiations]: #limitations
