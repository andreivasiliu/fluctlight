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
* typed-index-collections: 
