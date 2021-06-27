# nucleoid-persistence-backend (name WIP)
HTTP-based REST API for per-player, per-minigame key-value storage based on MongoDB.

## What can be stored?
Currently, the API can store:
- player uuid -> username 
- per-player, per-minigame statistic storage.
  - It currently supports storing a total or a rolling average integer value (returns a float for calculated average)

## Authentication
In order to allow this API to be exposed for public read access, certain endpoints require an authentication token in order to make successful requests.
Authentication tokens are stored in the `config.json` file, and on first run, a random 64 character string is generated as a default token. Tokens can simply be added or removed from the `server_tokens` option in order to create new tokens or invalidate old ones.

Authentication tokens should be passed in the `Authorization` HTTP header on every request to an authenticated endpoint. Endpoints that require authentication are marked below with a (*). If a request is missing the header, it will receive a `400 Bad request`, and if it has an invalid token in the `Authorization` header, it will receive a `401 Unauthorized` error.

## Statistic storage
The player statistic storage allows the following types of statistic to be stored:
- Raw value (stored as an `int`)
- Rolling average (stored as a `total` and `count`)

## REST API
### GET `/player/{uuid}`
#### Path parameters
| Name | Type | Description |
| --- | --- | --- |
| `uuid` | `UUID` | The player UUID to look up |

#### Response body
| Name | Type | Description |
| --- | --- | --- |
| `uuid` | `UUID` | The UUID of the player |
| `username` | `String?` | The player's username, if known, will be missing if not |

### PUT `/player/{uuid}` (*)
#### Path parameters
| Name | Type | Description |
| --- | --- | --- |
| `uuid` | `UUID` | The player UUID to update |

#### Request body
| Name | Type | Description |
| --- | --- | --- |
| `username` | `String` | The player's username, to be updated in the database

#### Response
This endpoint returns 204 no content on a successful request

### GET `/player/{uuid}/stats/{namespace}`
#### Path parameters
| Name | Type | Description |
| --- | --- | --- |
| `uuid` | `UUID` | The player UUID to lookup stats for |
| `namespace` | `String` | The namespace of stats to lookup, typically the name/mod id of the minigame; eg. `bed-wars` |

#### Response body
The response body is a `Map<String, float>` containing the values of all known statistics for the player. If the statistic is a raw value, it will simply be returned, and if it is a rolling average, then the calculated average will be returned.

### POST `/stats/upload` (*)
Should be called by the minigame server after a game has finished, to upload the stats for players in that game.

#### Request body
| Name | Type | Description |
| --- | --- | --- |
| `server_name` | `String` | Name of the server uploading the bundle; eg. `play` (currently unused by the backend) |
| `namespace` | `String` | The namespace of the game; eg `bed-wars` |
| `stats` | `Map<UUID, Map<String, UploadStat>>` | An object containing all stats for this game, by player. Note: statistic ids cannot contain '.'s |

#### `UploadStat` type
| Name | Type | Description |
| --- | --- | --- |
| `type` | `String` | The type of statistic this is, currently either `value` or `rolling_average` |
| `value` | See `Stat types` |

#### Stat types
| Name | Value type |
| --- | --- |
| `int_total` | `int` |
| `int_rolling_average` | `int` |
| `float_total` | `float` or `double` |
| `float_rolling_average` | `float` or `double` |

### Example payload
```json
{
  "server_name": "play",
    "namespace": "example-game",
    "stats": {
      "07e92b46838640678f728ab96e606fb7": {
      "example-1": {
        "type": "int_value",
        "value": 10
      },
      "example-2": {
        "type": "float_rolling_average",
        "value": 15.2
      }
    }
  }
}
```
