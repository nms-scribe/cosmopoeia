pub(crate) mod random_points;
pub(crate) mod triangles;
pub(crate) mod voronoi;
pub(crate) mod terrain;
pub(crate) mod beziers;
pub(crate) mod tiles;
pub(crate) mod climate;
pub(crate) mod water_flow;
pub(crate) mod water_fill;
pub(crate) mod rivers;
pub(crate) mod water_distance;
pub(crate) mod grouping;
pub(crate) mod biomes;
pub(crate) mod population;
pub(crate) mod naming;
pub(crate) mod culture_sets;
pub(crate) mod colors;
pub(crate) mod cultures;
pub(crate) mod towns;
pub(crate) mod nations;
pub(crate) mod subnations;
pub(crate) mod curves;

// FUTURE: It might make some of the code easier to work with if there were an Algorithm trait, and each of the algorithms are structs, which you have to fill with their dependencies, before calling a simple 'run(progress)' or something like that. Then I can break some of the more complex algorithms into simpler functions. The only issue are the fact that I'll have to borrow individual properties as mutable at the same time. But that might force me to separate my code better.
// -- another thing this can allow: for related algorithms, I can have 'from' functions which grab the input/output from a previous algorithm. so it can be more easily re-used.