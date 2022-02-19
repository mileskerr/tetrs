simple terminal based tetris clone written to learn rust

controls:

left/right/down to move

up to rotate clockwise

h to hold

esc to pause

q to quit

Scoring is based on NES tetris, while leveling, lock delay, and the ability to hold pieces are from more modern tetris games. 
I've forgone SRS in favor of a much more rudimentary rotation system that that just tests every 1 block offset and picks the first valid one.
Tetrominos are randomized using the random bag system rather than pure randomization, except for the first tetromino which is always a Z piece, because fuck you.



code is pretty messy, I'll probably work on cleaning it up later
