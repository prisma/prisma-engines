use prisma_models::PrismaValue;
use query_core::{
    constants::custom_types,
    response_ir::{Item, ItemRef, Map},
};
use request_handlers::{GQLBatchResponse, GQLResponse, PrismaResponse};

pub struct JsonResponse;

impl JsonResponse {
    /// Translates a GraphQL response to a JSON response. This is used to keep the same test-suite running on both protocols.
    /// JSON responses returns type-hinted scalars. This module mostly _removes_ those type-hints so that we can work with
    /// consistent data regardless of which protocol is being used to run the tests.
    pub fn from_graphql(response: PrismaResponse) -> PrismaResponse {
        if response.has_errors() {
            return response;
        }

        match response {
            PrismaResponse::Single(single) => PrismaResponse::Single(graphql_response_to_json_response(single)),
            PrismaResponse::Multi(batch) => {
                let responses: Vec<_> = batch
                    .into_responses()
                    .into_iter()
                    .map(graphql_response_to_json_response)
                    .collect();

                let mut new_batch = GQLBatchResponse::default();
                new_batch.insert_responses(responses);

                PrismaResponse::Multi(new_batch)
            }
        }
    }
}

fn graphql_response_to_json_response(single: GQLResponse) -> GQLResponse {
    let data = graphql_map_to_json_map(single.into_data());

    GQLResponse::new(data)
}

fn graphql_item_to_json_item(item: Item) -> Item {
    match item {
        Item::Value(pv) => Item::Value(unwrap_tagged_value(pv)),
        Item::Map(map) => Item::Map(graphql_map_to_json_map(map)),
        Item::List(list) => Item::List(
            list.into_iter()
                .map(graphql_item_to_json_item)
                .collect::<Vec<_>>()
                .into(),
        ),
        Item::Ref(ref_item) => graphql_item_to_json_item(item_ref_to_owned_item(ref_item)),
        _ => item,
    }
}

/// The serialization layer can use an `Item::Ref` to allow multiple parent records
/// to claim the same item without copying data. Given that we cannot mutate those Arcs,
/// we have to clone all those Refs so that we own them.
/// This is only ok because we're doing that in our test environment.
fn item_ref_to_owned_item(item_ref: ItemRef) -> Item {
    let item_ref = item_ref.as_ref();

    match item_ref {
        Item::Map(map) => Item::Map(map.to_owned()),
        Item::List(list) => Item::List(list.to_owned()),
        Item::Value(val) => Item::Value(val.to_owned()),
        Item::Json(json) => Item::Json(json.to_owned()),
        Item::Ref(nested_ref) => item_ref_to_owned_item(nested_ref.clone()),
    }
}

fn graphql_map_to_json_map(map: Map) -> Map {
    let mut res: Map = Map::new();

    for (k, item) in map {
        let new_item = graphql_item_to_json_item(item);

        res.insert(k, new_item);
    }

    res
}

fn unwrap_tagged_value(pv: PrismaValue) -> PrismaValue {
    match pv {
        PrismaValue::Object(obj) if is_tagged_value(&obj) => {
            let mut iter = obj.into_iter();
            iter.next();

            iter.next().unwrap().1
        }
        pv => pv,
    }
}

fn is_tagged_value(obj: &[(String, PrismaValue)]) -> bool {
    if obj.len() != 2 {
        return false;
    }

    let mut iter = obj.iter();
    let (key1, _) = iter.next().unwrap();
    let (key2, _) = iter.next().unwrap();

    key1 == custom_types::TYPE && key2 == custom_types::VALUE
}
