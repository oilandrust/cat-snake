pub const REACH_GOAL_FALLING: &str = "...........
.....AX....
...aaa.....
...#.......";

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

pub const SNAKES_BUG: &str = "X......
.....B..
..aaAb..
..abbb.
...b....
########";

pub const ACTIVATE_ON_EAT: &str = ".......
.......X....
...aaA.oo...
###########";

pub const TEST_LEVELS: [&str; 4] = [
    SNAKES_BUG,
    FALL_ON_SPIKE,
    FALL_ON_SNAKE_BUG,
    ACTIVATE_ON_EAT,
];
