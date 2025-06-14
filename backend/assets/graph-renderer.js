// graph-renderer.js
const GRAPH_ENDPOINT = "/graph/layout";
const POLL_INTERVAL_MS = 1000; // Poll every 1 second

const margin = { top: 20, right: 20, bottom: 20, left: 20 };
const width = 800 - margin.left - margin.right; // SVG width
const height = 600 - margin.top - margin.bottom; // SVG height

// Create the SVG container
const svg = d3
    .select("#graph-container")
    .append("svg")
    .attr("width", width + margin.left + margin.right)
    .attr("height", height + margin.top + margin.bottom)
    .append("g")
    .attr("transform", `translate(${margin.left},${margin.top})`);

// Scales for mapping data positions to SVG coordinates
let xScale = d3.scaleLinear();
let yScale = d3.scaleLinear();

// Elements for nodes and links
let linkGroup = svg.append("g").attr("class", "links");
let nodeGroup = svg.append("g").attr("class", "nodes");

function updateGraph(graphData) {
    if (!graphData || !graphData.nodes || !graphData.edges) {
        console.warn("Invalid graph data received:", graphData);
        return;
    }

    // Create a map for quick node lookup by ID
    const nodesById = new Map(graphData.nodes.map((d) => [d.node.id, d]));

    // Extract x and y positions from nodes for scaling
    const allX = graphData.nodes.map((d) => d.pos[0]);
    const allY = graphData.nodes.map((d) => d.pos[1]);

    // Calculate min/max for scaling. Add a little padding to the domain.
    const xMin = d3.min(allX);
    const xMax = d3.max(allX);
    const yMin = d3.min(allY);
    const yMax = d3.max(allY);

    const xPadding = (xMax - xMin) * 0.1; // 10% padding
    const yPadding = (yMax - yMin) * 0.1;

    xScale.domain([xMin - xPadding, xMax + xPadding]).range([0, width]);
    yScale.domain([yMin - yPadding, yMax + yPadding]).range([height, 0]); // Invert Y for SVG coordinates

    // --- Update Links ---
    const links = linkGroup
        .selectAll(".link")
        // The key function now uses the 'id' from the third element of the edge array
        .data(graphData.edges, (d) => `edge-${d[2].id}`);

    // Exit
    links.exit().remove();

    // Enter
    const newLinks = links.enter().append("line").attr("class", "link");

    // Update + Enter
    const allLinks = newLinks
        .merge(links)
        .attr("x1", (d) => xScale(nodesById.get(d[0]).pos[0])) // d[0] is source_id
        .attr("y1", (d) => yScale(nodesById.get(d[0]).pos[1]))
        .attr("x2", (d) => xScale(nodesById.get(d[1]).pos[0])) // d[1] is target_id
        .attr("y2", (d) => yScale(nodesById.get(d[1]).pos[1]));

    // --- Update Nodes ---
    const nodes = nodeGroup
        .selectAll(".node")
        .data(graphData.nodes, (d) => d.node.id); // Key for unique nodes

    // Exit
    nodes.exit().remove();

    // Enter
    const newNodeGroup = nodes.enter().append("g").attr("class", "node");

    newNodeGroup.append("circle").attr("r", 5); // Default radius

    newNodeGroup.append("text").text((d) => d.node.data.label);

    // Update + Enter (position nodes)
    const allNodes = newNodeGroup
        .merge(nodes)
        .attr(
            "transform",
            (d) => `translate(${xScale(d.pos[0])},${yScale(d.pos[1])})`
        );

    // Update text (in case labels change)
    allNodes.select("text").text((d) => d.node.data.label);
}

// The rest of the script remains the same
async function fetchDataAndRender() {
    try {
        const response = await fetch(GRAPH_ENDPOINT);
        if (!response.ok) {
            throw new Error(`HTTP error! status: ${response.status}`);
        }
        const data = await response.json();
        updateGraph(data);
    } catch (error) {
        console.error("Error fetching graph data:", error);
    }
}

// Initial fetch and then poll
fetchDataAndRender(); // Fetch immediately on load
setInterval(fetchDataAndRender, POLL_INTERVAL_MS);
