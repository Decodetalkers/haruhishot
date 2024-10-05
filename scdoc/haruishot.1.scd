haruhishot(1)

# NAME

haruhishot - screenshot program for wlroots

# DESCRIPTION

Screenshot for wlroots

# COMMANDS

*list_outputs (--list_outputs) (-L)*
	Get the Display Information

*slurp (--slurp) (-S)* [--stdout]
	Take screenshot for a center rigon
	
	Examples:
		```
		haruhishot -S (slurp -d)
		```
	Use "--stdout: will print the image to console, you can copy it to clipboard
	Examples:
		```
		haruhishot -S (slurp -d) --stdout | wl-copy
		```

*global* [--stdout]
	Take screenshot for all there screen, this will combine all screens together
	"--stdout" is the same

*gui*
	This will open a qt fontend for you to take screenshot

*output (--output) (-O)* [--stdout]
	Choose screen to takescreen. There is always screen name after `-O`, you can
	get it with `swaymsg` or `list_outputs` option. If you do not give a variable
	to it, it will open a cli menu for you to select

	"--stdout" is the same as above
	
	Examples:
	```
	haruhishot -O
	haruhishot -O eDP-1
	```

*color (--color) (-C)*
	Get color

	Examples:
	```
	haruhishot -C (slurp -p)
	```
