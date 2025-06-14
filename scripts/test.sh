#!/bin/bash

# Base URL for the API
BASE_URL="http://localhost:8080"

echo "--- Starting API Tests ---"

echo "--- Test 1: Get initial nodes (should be empty) ---"
http GET $BASE_URL/graph
echo -e "\n"

echo "--- Test 2: Add two nodes and one edge ---"
http POST $BASE_URL/graph \
  nodes:='[{"id": "0", "data": {"label": "Node A"}}, {"id": "1", "data": {"label": "Node B"}}]' \
  edges:='[{"a": "0", "b": "1", "edge": {"id": "0"}}]'
echo -e "\n"

echo "--- Test 3: Get nodes after adding (should show Node A and Node B) ---"
http GET $BASE_URL/graph
echo -e "\n"

echo "--- Test 4: Add another node and an edge connecting to an existing node ---"
http POST $BASE_URL/graph \
  nodes:='[{"id": "2", "data": {"label": "Node C"}}]' \
  edges:='[{"a": "1", "b": "2", "edge": {"id": "1"}}]'
echo -e "\n"

echo "--- Test 5: Get all nodes again (should show Node A, Node B, and Node C) ---"
http GET $BASE_URL/graph
echo -e "\n"

echo "--- Test 6: Attempt to add nodes with missing data (expecting error or malformed request) ---"
echo "  Note: The server's error handling for malformed requests is not explicitly defined in the Rust code."
http POST $BASE_URL/graph \
  nodes:='[{"id": "3"}]'
echo -e "\n"

echo "--- Test 7: Add a node without any edges ---"
http POST $BASE_URL/graph \
  nodes:='[{"id": "4", "data": {"label": "Node D"}}]' \
  edges:='[]'
echo -e "\n"

echo "--- Test 8: Get nodes to verify Node D was added ---"
http GET $BASE_URL/graph
echo -e "\n"

echo "--- API Tests Complete ---"

