        // TODO: "You can also find the dual (ie. Voronoi diagram) just by computing the circumcentres of all the triangles, and connecting any two circumcentres whose triangles share an edge."
        // - Given a list of (delaunay) triangles where each vertice is one of the sites
        // - Calculate a map (A) of triangle data with its circumcenter TODO: How?
        // - Calculate a map (B) of sites with a list of triangle circumcenters TODO: How?
        // - for each site and list in B
        //   - if list.len < 2: continue (This is a *true* edge case, see below) TODO: How to deal with these?
        //   - vertices = list.clone()
        //   - sort vertices in clockwise order TODO: How? (See below)
        //   - vertices.append(vertices[0].clone()) // to close the polygon
        //   - create new polygon D(vertices)
        //   - sample the elevation from the heightmap given the site coordinates
        //   - add polygon D to layer with site elevation attribute
    
        // TODO: Finding the Circumcenter: https://en.wikipedia.org/wiki/Circumcircle#Cartesian_coordinates_2
    
        // TODO: Finding the map of sites to triangle circumcenters:
        // - create the map
        // - for each triangle in the list of triangles
        //   - for each vertex:
        //     - if the map has the site, then add this triangle's circumcenter to the list
        //     - if the map does not have the site, then add the map with a single item list containing this triangle's circumcenter
    
        // TODO: Actually, we can simplify this: when creating the map, just add the circumcenter vertex
    
        // TODO: Sorting points clockwise:
        // - https://stackoverflow.com/a/6989383/300213 -- this is relatively simple, although it does take work.
        // - Alternatively, there is a concave hull in gdal which would work, except it's not included in the rust bindings.
    
    
        // TODO: I think I'm going to rethink this, since I'm having to store things in memory anyway, and the originally generated points aren't
        // necessarily the ones I get from the database, the algorithms should deal with the types themselves and only occasionally the data files.
        // Basically:
        // - generate_random_points(extent NOT the layer) -> Points
        // - calculate_delaunay(points) -> triangles
        // - calculate_voronoi(triangles) -> voronois (polygons with "sites")
        // - create_tiles(voronois,heightmap) -> create layer with the voronoi polygons, sampling the elevations from the heightmap
        // - however, if I'm using the gdal types (until I can get better support for the geo_types), I can have stuff that will write the data to layers for visualization
    
