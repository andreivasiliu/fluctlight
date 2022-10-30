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
* im: immutable data structures

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
* Idea: Perhaps something with globally unique state branch IDs
* Idea: State map is an array, each node holds branch extremities
  * Extremities hold a past-going linked list of dates and values
  * New extremity added when this key is modified on a branch
    * Conversely, if this key was not modified, the branch is skipped
  * Merged branches inherit the longest branch's name
  * Branch points include a replay of each short branch's history
  * Extremities can be reordered to make the main branch be the first
  * This perhaps models a tiny subset replica of the entire state graph
    * Except that branches are never merged, and remain an extremity
  * Use a[depth].b[depth].c branch names in case b or c are not there
    * Or have a map of branches to their branch source

Queries an event/state must support:
* Backfill over federation
  * Was the room visibility public at this state
  * Was any user of the request server joined at this state
    * Alternatively: did the server have any users joined at this state
* Normal operations
  * Was the sender joined at this state
* Relay presence/typing EDUs
  * None (only needs the room's latest state)
* Relay join event
  * What servers have a user joined at this state
  * Might also work to just record this state

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
* Client sends a message
  * Incoming client request
  * Incoming client response
  * Other clients' incoming syncs are triggered
  * Background outgoing federation requests
    * Outgoing retry timers created on failure
* Federated server sends incoming PDU
  * Incoming federated request
    * Outgoing key query
  * Outgoing auth chain event retrieval (multiple)
    * Outgoing key query (multiple)
  * Outgoing state retrieval (multiple)
    * Outgoing key query (multiple)
  * Incoming federated response (could perhaps be done earlier)
  * Other clients' incoming syncs are triggered
* Client wants to sync from a room
  * Incoming client request
  * Potential outgoing backfill request
    * Outgoing key query
  * Potential outgoing state ID request
    * Outgoing key query
  * Potential outgoing event request
    * Outgoing key query
  * Incoming client response

Handling flows:
* Needs to honor the "Logic core, IO shell" section above
* For out-of-band requests
  * Logic core can maintain a list of named outgoing operations
  * At startup/restart, it sends these to the IO shell
  * IO shell likewise maintains a list of named outgoing requests
* For in-band requests
  * Logic core can reply with a redirection request
* For timer-based operations
  * Possibly similar design as out-of-band requests

Response borrowing:
* Use a request.response(..data..) API in order to borrow data
* Routes can require BuiltResponse as an output

Trusting federation:
* Federated room state cannot be fully validated
* Maintain a trust-depth
  * Everything with bigger depth is validated
  * Everything with smaller depth is queried and blindly trusted
* Backfill can assign unknown state IDs at branches
  * This should detect many same-state merges of non-state messages

PDU disk storage:
* Need to choose between:
  * Super-compact: store interned strings
  * Super-compatible: store exact JSON replica
* Likely a choice that can be changed later
* JSON replica store:
  * JSON stream of individual PDUs
  * Separate index with pointers to file and start/end offsets
  * Separate map with event_id to index
  * Restart zlib/deflate compression for each PDU with preset dictionary
* Would be nice to store:
  * The original PDU
  * The origin and time it was received
  * In what way it was received (join state, transaction, backfill)
  * The computed event_id (even if it's wrong)
  * Which events it merges state from (as an optimization)
* Would be nice to organize events in:
  * Join/genesis event, and/or smallest depth trusted event
  * Trusted state at event (perhaps only as state IDs?)
  * State events or state merging events
  * Normal events

Road to a public federation listener:
* Must correctly answer key requests
  * Must check against local Synapse
  * Must check against federation tester
* Must check with homeserver devs if plain/text 501s are acceptable
* Must join only v5/v6 rooms
  * May implement runtime check when redacting
* Should figure out PDU storage
  * May index and skip loading non-state PDUs
* Must track PDU origin
* Must perform new join experiment
* Must figure out packaging and deployment
* Should fix event ID calculation
* May implement redaction (this could allow public read-only access)
* May implement profile and device list query APIs
* Should set display name on the join event
* Should display memory usage
* Should be able to drop non-state PDUs from memory
* Must rename key to have Fluctlight-specific prefix
