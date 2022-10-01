# Random collection of design thoughts

HotString/ColdString:
* 4 bits for location
* Can resolve but needs all string storages
* Thread-local L1 cache

Intern everything
* State machine for reaching IDs
  * Can split bytes into 16*16 bits
  * Can maybe do something with base64
* Static strings for JSON keys

Store current state in RoomServer memory
* What to do with old state?
  * Maybe figure out join/leave ranges for users somehow
  * Is it possible to say "between X and Y" in a graph?

Frozen PDU blocks
* For prev/next tokens, use block ID + ID of PDU index inside block

Rooms servers
* Perhaps as full-blown servers with a proxy in front
* Config with room-glob=local/fork/proxy
* Shadow mode for development?
* Potentially problematic:
  * Sending m.presence EDUs to co-shared users/servers
  * Retrieve single event
* Secondary servers should have no configuration and allow migrations

Disk controls
* Store PDU blocks and media blocks separately

Quick JSON deserialization
* SortedMap<&[u8]>, using bumpalo's vec
* Deserializes a map into a vec
* Errors if items are added in non-sorted order
* Can use a binary search on keys
* Could maybe have two vecs (keys/values)

Memory shenanigans:
* bump: request memory pool
* vec-collections: SmallVec-based set and map
* elsa: For interning, append-only collections
* slotmap: For interning, generational O(1) indexes
* weak-table: Maybe for some shenanigans?
* servo's interned arc string
* SortedMap<&[u8]>: an already-sorted vec, used as a map
* ouroboros: a PDU's RawValue plus deserialized PDU referencing it
* append-only-vec: can append with shared reference
* IntStr/ArcStr combo: best of both worlds

Logic core, IO shell:
* Everything in the module must not be async
* Connections are only allowed in the module loader
* Module must be reloadable in the middle of a join request, successfully
  * Module must not store intermediate temporary state
  * Shell must not store deserialized data
* Incoming connection:
  * Client connects, shell asks module to process request
  * Module says "nope, get keys first"
  * Shell gets keys, shell asks module to process keys
  * Shell asks module to process initial request again
* Alternatively, special-case authentication
  * This means only the request header is needed
  * Can drop super-large requests immediately if auth fails
* Outgoing join request
  * Client asks to join, shell asks module to process request
  * Module says "nope, get room directory first", shell retries
  * Module says "nope, get make_join first", shell retries
  * Module says "nope, get send_join_first"
  * Shell connects, gets 160MB response, asks module to process
  * Module says "processed, but get keys"
  * Shell gets keys, asks module to process request

Data structures:
* State maps need two features:
  * Ability to iterate over the state as it was at a specific point in time
  * Ability to locate a key at a specific point in time
* Iteration should be O(n), lookup should be O(log n) at most
* Idea: List of cells with limited lifecycle
  * Each cell's "next" pointer has two time/revision thresholds
  * Before the first threshold, nothing is after it
  * Before the second threshold, something is after it
  * After the second threshold, something else is after it
  * This can "skip" deleted nodes
  * However, it doesn't work if the next node is repeatedly deleted
  * Workaround is to recursively replace the cells before it
* Idea: List of cells with full history for each cell
  * The idea is that cells can never be deleted, only updated
* Idea: State map is an array, state string interned ID is the index
  * All state strings need to be interned, and they can never be deleted
  * Therefore, they are stable array indexes
  * The individual cells still need a history of modifications
  * Iteration would be O(n), lookup would be mostly O(1)

Flows:
* Client asks to join over federation
  * Incoming client request
  * Outgoing directory
    * Outgoing key query
  * Outgoing make_join
    * Outgoing key query
  * Outgoing send_join
    * Parse room state
    * Outgoing key query
    * Outgoing missing events?
  * Incoming client response
* Federated server sends incoming PDU
  * Incoming federated request
    * Outgoing key query
  * Outgoing auth chain event retrieval (multiple)
    * Outgoing key query (multiple)
  * Outgoing state retrieval (multiple)
    * Outgoing key query (multiple)
  * Incoming federated response (could perhaps be done earlier)

Trusting federation:
* Federated room state cannot be fully validated
* Maintain a trust-depth
  * Everything with bigger depth is validated
  * Everything with smaller depth is queried and blindly trusted
* Backfill can assign unknown state IDs at branches
  * This should detect many same-state merges of non-state messages

PDU storage:
* Need to choose between:
  * Super-compact: store interned strings
  * Super-compatible: store exact JSON replica
* Likely a choice that can be 
* JSON replica store:
  * JSON stream of individual PDUs
  * Separate index with pointers to file and start/end offsets
  * Separate map with event_id to index
