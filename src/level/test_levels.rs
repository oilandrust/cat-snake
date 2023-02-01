pub const FALL_ON_SPIKE: &str = "....X..
..aA...
.+#....
....++.";

pub const FALL_ON_SNAKE_BUG: &str = "...X.
......
.B....
.baA..
.bb...
..#...";

pub const BUG_SNAKES_ON_TOP: &str = "X......
.....B..
..aaAb..
..abbb.
...b....
########";

pub const ACTIVATE_ON_EAT: &str = ".......
.......X....
...aaA.oo...
###########";

pub const BUG_EXIT_LEVEL_ON_FALL: &str = "......
...............
....aAX......
..###.........
..............
.............";

pub const TEST_EXIT_ANIM: &str = "......
...............
bbBaaAX......
..#a#.........
..aa..........
.............";

pub const TEST_LEVELS: [&str; 5] = [
    TEST_EXIT_ANIM,
    BUG_SNAKES_ON_TOP,
    FALL_ON_SPIKE,
    FALL_ON_SNAKE_BUG,
    ACTIVATE_ON_EAT,
];