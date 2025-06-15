import { select } from "d3-selection"; // For d3.select
import { ScaleLinear, scaleLinear } from "d3-scale"; // For d3.scaleLinear
import { min, max } from "d3-array"; // For d3.min, d3.max

// Define interfaces for graph data
interface NodeData {
    node: {
        id: string;
        data: {
            label: string;
            // Add other properties if they exist in your node data
        };
        // Add other properties if they exist in your node structure
    };
    pos: [number, number]; // [x, y] coordinates
}

interface EdgeData {
    0: string; // Source Node ID
    1: string; // Target Node ID
    2: {
        id: string; // Edge ID
        // Add other properties if they exist in your edge data
    };
}

interface GraphData {
    nodes: NodeData[];
    edges: EdgeData[];
    creation_time: number;
}

const GRAPH_ENDPOINT: string = "/graph/layout";
const POLL_INTERVAL_MS: number = 100;

const margin = { top: 20, right: 20, bottom: 20, left: 20 };
const width: number = 800 - margin.left - margin.right; // SVG width
const height: number = 600 - margin.top - margin.bottom; // SVG height

// Create the SVG container
const svg = select("#graph-container")
    .append("svg")
    .attr("width", width + margin.left + margin.right)
    .attr("height", height + margin.top + margin.bottom)
    .append("g")
    .attr("transform", `translate(${margin.left},${margin.top})`);

svg.append("defs")
    .append("marker")
    .attr("id", "arrowhead")
    .attr("viewBox", "0 -5 10 10") // Adjust viewBox based on your arrow size
    .attr("refX", 8) // This positions the tip of the arrow at the end of the line
    .attr("refY", 0)
    .attr("orient", "auto")
    .attr("markerWidth", 10) // Size of the marker
    .attr("markerHeight", 10)
    .append("path")
    .attr("d", "M0,-5L10,0L0,5") // Path for a simple arrowhead
    .attr("fill", "#999"); // Color of the arrowhead, match your link color

// Scales for mapping data positions to SVG coordinates
let xScale: ScaleLinear<number, number> = scaleLinear();
let yScale: ScaleLinear<number, number> = scaleLinear();

// Elements for nodes and links
let nodeGroup = svg.append("g").attr("class", "nodes");
let linkGroup = svg.append("g").attr("class", "links");

let lastCreationTime: number | null = null;

/**
 * Updates the D3 graph visualization based on the provided graph data.
 * @param graphData The data containing nodes and edges to render.
 */
function updateGraph(graphData: GraphData): void {
    if (!graphData || !graphData.nodes || !graphData.edges) {
        console.warn("Invalid graph data received:", graphData);
        return;
    }

    // Create a map for quick node lookup by ID
    const nodesById = new Map<string, NodeData>(
        graphData.nodes.map((d) => [d.node.id, d])
    );

    // Extract x and y positions from nodes for scaling
    const allX: number[] = graphData.nodes.map((d) => d.pos[0]);
    const allY: number[] = graphData.nodes.map((d) => d.pos[1]);

    // Calculate min/max for scaling. Add a little padding to the domain.
    const xMin: number = min(allX) ?? 0; // Use nullish coalescing to provide default if array is empty
    const xMax: number = max(allX) ?? 0;
    const yMin: number = min(allY) ?? 0;
    const yMax: number = max(allY) ?? 0;

    const xPadding: number = (xMax - xMin) * 0.1; // 10% padding
    const yPadding: number = (yMax - yMin) * 0.1;

    xScale.domain([xMin - xPadding, xMax + xPadding]).range([0, width]);
    yScale.domain([yMin - yPadding, yMax + yPadding]).range([height, 0]); // Invert Y for SVG coordinates

    // --- Update Links ---
    const links = linkGroup
        .selectAll<SVGLineElement, EdgeData>(".link") // Explicitly type the selection
        // The key function now uses the 'id' from the third element of the edge array
        .data(graphData.edges, (d) => `edge-${d[2].id}`);

    // Exit
    links.exit().remove();

    // Enter
    const newLinks = links.enter().append("line").attr("class", "link");

    // Update + Enter
    const allLinks = newLinks
        .merge(links)
        .attr("x1", (d: EdgeData) => xScale(nodesById.get(d[0])!.pos[0])) // d[0] is source_id
        .attr("y1", (d: EdgeData) => yScale(nodesById.get(d[0])!.pos[1]))
        .attr("x2", (d: EdgeData) => xScale(nodesById.get(d[1])!.pos[0])) // d[1] is target_id
        .attr("y2", (d: EdgeData) => yScale(nodesById.get(d[1])!.pos[1]))
        .attr("marker-end", "url(#arrowhead)");

    // --- Update Nodes ---
    const nodes = nodeGroup
        .selectAll<SVGGElement, NodeData>(".node") // Explicitly type the selection
        .data(graphData.nodes, (d) => d.node.id); // Key for unique nodes

    // Exit
    nodes.exit().remove();

    // Enter
    const newNodeGroup = nodes.enter().append("g").attr("class", "node");

    newNodeGroup.append("circle").attr("r", 5); // Default radius

    newNodeGroup
        .append("text")
        .text((d: NodeData) => d.node.data.label)
        .attr("transform", (d: NodeData) => `translate(7, 0)`);

    // Update + Enter (position nodes)
    const allNodes = newNodeGroup
        .merge(nodes)
        .attr(
            "transform",
            (d: NodeData) =>
                `translate(${xScale(d.pos[0])},${yScale(d.pos[1])})`
        );

    // Update text (in case labels change)
    allNodes.select("text").text((d: NodeData) => d.node.data.label);
}

/**
 * Fetches graph data from the endpoint and renders it.
 */
async function fetchDataAndRender(): Promise<void> {
    try {
        const response: Response = await fetch(GRAPH_ENDPOINT);
        if (!response.ok) {
            throw new Error(`HTTP error! status: ${response.status}`);
        }
        const data: GraphData = await response.json();
        if (lastCreationTime === null) {
            lastCreationTime = data.creation_time;
        } else if (data.creation_time !== lastCreationTime) {
            window.location.reload();
            return;
        }
        updateGraph(data);
    } catch (error: any) {
        // Use 'any' or more specific error types if known
        console.error("Error fetching graph data:", error);
    }
}

// Initial fetch and then poll
fetchDataAndRender(); // Fetch immediately on load
setInterval(fetchDataAndRender, POLL_INTERVAL_MS);
