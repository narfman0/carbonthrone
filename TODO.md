* the researcher does not recover ap at the start of the turn. it should be full. fix.
* when moving characters, disregard facing the nearest enemy. remove that logic to simplify.
* during exploration, disregard ap, and allow for free movement. multiple tiles at once is fine.
* during movement, for battle and exploration, we can't move on or through to obstacle tiles. use a pathfinding altorithm to construct a path to follow to the destination, animate it with the player moving at the current speed. then stop.
* during dialog, we should show the current zone
* the health bar is diagonal. make it straight. make it the width of the tile.
* the position of the text and level is incorrect. attach it to, and put it above, the health bar
* always show skills, even if they are unavailable. gray them out and make them inactive when unusable, but always show them.
* dr orin can only heal at level 1. give her a light ranged attack ability.
* dr orin's heal ability requires a target in order to heal. make sure the player clicks the target before resolving the ability.
