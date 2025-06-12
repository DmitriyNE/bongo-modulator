# Bongo modulator

This is a thingy that modulates bongo cat intensity on the hyprlock lockscreen.
The bongo cat is displayed by hyprlocks image element. Since it does not support
images, we use reload_cmd to give it a new frame of the bongo cat each update.

Thus, our bongo-modulator should perform 3 functions.
1. Detect hyprlock presence and spam it with USR2 signals so it updates.
2. Provide an endpoint to the hyprlock so it can ask for a next image.
3. Modulate USR2 frequency based on different events.
3.1. Manual mode, where i can specify fps from cmdline
3.2. AI mode, where the cat is modulated by the number and closedness of people,
     seen by a webcam.

Project language is Rust.
