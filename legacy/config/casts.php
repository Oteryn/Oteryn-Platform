<?php

return [
    'enabled' => false,
    // Exact verified TLS subject => canonical Game node UUID. Deployment-owned,
    // separate from Gateway credentials; never populated from request headers.
    'nodes' => [],
    'routes' => [],
];
