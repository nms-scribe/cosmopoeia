max_habitability = maximum of tiles habitability
by_habitability = s = |tile| tile.habitability
by_shore_distance = t = |tile| tile.shore_distance
by_elevation = h = |tile| tile.elevation
by_normalized_habitability = n = |tile| ((tile.habitability / max_habitability) * 3).ceil(); // normalized cell score
by_temperature_difference(goal) = td = |tile,goal| (tile.temperature - goal).abs() + 1; // temperature difference fee
by_biome(biomes,fee) = bd = |tile,biomes,fee=4| if biomes.contains(tile.biome) { 1 } else { fee }; // biome difference fee
by_sea_coast(fee) = sf = |tile,fee=4| tile.closest_water && closest_water.type != "lake" ? 1 : fee; // not on sea coast fee
negate(sort) = -?
multiply(sore,sort)
ratio(sort,sort)
add(sort,sort)
pow(sort,float)


TODO: These are the different types of sorts in the culture sets. Turn them into enums.

negate(by_habitability)
(by_elevation * by_shore_distance) / by_biomes(biomes,fee)
(by_normalized_habitability / by_biomes(biomes,fee)) * by_shore_distance
(by_normalized_habitability / by_temperature_difference(goal) / by_biomes(biomes,fee)) * by_shore_distance
(by_normalized_habitability / by_temperature_difference(goal) / by_sea_coast(fee)) * by_elevation
(by_normalized_habitability / by_temperature_difference(goal)) * by_elevation
(by_normalized_habitability / by_temperature_difference(goal)) * by_sea_coast(fee)
(by_normalized_habitability / by_temperature_difference(goal)) * by_shore_distance
by_elevation * by_shore_distance
by_elevation / by_temperature_difference(goal)
by_elevation / by_temperature_difference(goal) / by_biomes(biomes,fee)
by_normalized_habitability / by_biomes(biomes,fee)
by_normalized_habitability / by_temperature_difference(goal)
by_normalized_habitability / by_temperature_difference(goal) ** 0.5 / by_biomes(biomes,fee)
by_normalized_habitability / by_temperature_difference(goal) / by_biomes(biomes,fee)
by_normalized_habitability / by_temperature_difference(goal) / by_biomes(biomes,fee) / by_shore_distance
by_normalized_habitability / by_temperature_difference(goal) / by_sea_coast(fee)
by_normalized_habitability / by_temperature_difference(goal) / by_sea_coast(fee) / by_shore_distance
by_normalized_habitability / by_temperature_difference(goal) / by_shore_distance
by_normalized_habitability + by_elevation
add(by_shore_distance,negate(by_habitability))
by_temperature_difference(goal)
by_temperature_difference(goal) / by_biomes(biomes,fee) / by_sea_coast(fee)