# Lib-Account

Lib-Account is part of the LevelCrush lib project.
Due to the database requirements in v2 it has been split off into its own repository so it can live
in a isolated space.

Lib account contains all functionality to run a login and auth server that uses
discord as the primary login and can link a bungie/twitch/etc account to it.

## Notes

* When running the server for the first time. If you dont already have the environment variables preconfigured it will
  intentionally crash. On first **fresh** run it will create entries in the __**application**__ server database. And
  give defaults where possible. The **server secret** is not a default that is generated and must be manually set. It is
  64 characters long.
* Since this is a library and not a binary.  [service-accounts](https://code.levelcrush.com/LevelCrush/service-accounts)
  is the primary user of
  this library and is intended to be very minimal since all responsible logic is kept here. service-accounts 