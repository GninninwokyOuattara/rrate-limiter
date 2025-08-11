local urls = {
    "/api/v1/fw",
    "/api/v1/swl",
    "/api/v1/swc",
    "/api/v1/tb",
    "/api/v1/lb",
}

-- Keep track of which URL to use next
local url_index = 1

-- The 'request' function is called for each HTTP request
function request()
    -- Get the current URL from the list
    local path = urls[url_index]
    
    -- Increment the index for the next request, and loop back if needed
    url_index = url_index + 1
    if url_index > #urls then
        url_index = 1
    end
    
    -- Return a formatted request string to wrk
    return wrk.format("GET", path)
end