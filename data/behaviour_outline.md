
# Triggers

## Terms:
- This card: The card with the behavior
- Activator: The card that triggered the behavior (What was summoned, what declared an attack, etc...)


    With all that was discussed, I wanted to further explore common 'match' cases, such as card types (Not category) and location.
    Consider the 'location'. We could use the term 'in' to specify a location constraint for THIS card (Not the activator).

```toml
in = "graveyard"
when = "owner:turn_start(ed?)"
and = { "counter:haunting:greater_than" = 3}
```

## Type
### Activator

This is the modifier that can optionally be placed before a trigger type.

- owned
  - owner - A potential alias to be used primarily with triggers that aren't associated with a card (turn start, draw card, etc...)
- opponent
- this
- either

### Respond to

- Summon 
  - will_be_summoned
  - has_been_summoned
  
- Cast
  - will_cast
  - has_cast

- Damage
  - will_attack
  - will_be_attacked
  - has_attacked
  - has_been_attacked
  - took_damage

- Destroy
  - has_destroyed
  - has_been_destroyed
  - has_defeated
  - has_been_defeated


    Note: Defeat refers to a card being destroyed during an attack


- Misc
  - was_drawn
  - draw_card
  - turn_end
  - turn_start
    - turn_started - Potential replacement? Maybe sounds better

# Actions


    The more I think about actions, the more I realize that they will need to be able to specify a target to perform
    the action on. Worth noting that the target could be multiply units, not just one. This may be difficult to implement.
    This could either be given in the same context object that the trigger used to decide if this action should fire,
    or selected based on some heuristics like random selection, opponent with the most health, etc.


- replace - Destroy target unit and replace it with a new one
- replace_many - Destroy a group of units and replace it with a new one
- replace_many_individual - Replace a group of units with a new group of units (One for each unit destroyed)
- add_types - Add some types to a card. This card could be this card, or it could be some other card.
- modify_defense - Change the defense of targeted units by the amount specified (+1, -5, =2, etc.)
- modify_health - ^ but health
- modify_attack - ^ but attack
- transfer_types - Append the types of one card onto another
- destroy - Destroy the selected card (May not be a unit! It could be an item, too)
- summon - Summon a new unit to the field. This could be summoning an existing unit or a completely new one.
    
    
    Summoning refers to the action of consuming Thaum and placing a unit on the field. Mana must be available for this to happen


- place - Place a new unit to the field. This does not consume Thaum


    There are many more actions I can think of but it is starting to sound very repetitive so I will forego the exhaustive list.
    We should talk more about how this may work as it seems like the actions will need a lot of context to construct.