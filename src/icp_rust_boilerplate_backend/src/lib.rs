//a canister to perform basic CRUD operations on timberyard management

#[macro_use]
extern crate serde;
use candid::{Decode, Encode};
use ic_cdk::api::time;
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{BoundedStorable, Cell, DefaultMemoryImpl, StableBTreeMap, Storable};
use std::{borrow::Cow, cell::RefCell};


type Memory = VirtualMemory<DefaultMemoryImpl>;
type IdCell = Cell<u64, Memory>;

#[derive(candid::CandidType, Clone, Serialize, Deserialize, Default)]
//we can only have the following timber types:
//1. cyprus
//2. pine
//3. oak
//4. cedar
//5. spruce

//we can only have the following timber sizes:
//1. 2x4
//2. 2x6
//3. 2x8
//4. 2x10
//5. 3x2
//6. 3x4
//7. 4x2
//8. 4x4
//9. 4x6
//10. 6x2
//11. 6x4
//12. 8x2
//13. 8x4
//14. 10x2
//15. 10x4

struct Timber {
    id: u64,
    timber_type: String,
    timber_size: String,
    quantity: u64,
    created_at: u64,
    updated_at: Option<u64>,
}

//sales struct
#[derive(candid::CandidType, Clone, Serialize, Deserialize, Default)]
struct Sales {
    id: u64,
    timber_id: u64,
    quantity: u64,
    price: u64,
    created_at: u64,
    updated_at: Option<u64>,
}

// a trait that must be implemented for a struct that is stored in a stable struct
impl Storable for Timber {
    //converts the struct to bytes
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        // attempt to serialize the struct using the Encode! macro
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

// another trait that must be implemented for a struct that is stored in a stable struct
impl BoundedStorable for Timber {
    const MAX_SIZE: u32 = 1024;
    const IS_FIXED_SIZE: bool = false;
}

// a trait that must be implemented for a struct that is stored in a stable struct
impl Storable for Sales {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

// another trait that must be implemented for a struct that is stored in a stable struct
impl BoundedStorable for Sales {
    const MAX_SIZE: u32 = 1024;
    const IS_FIXED_SIZE: bool = false;
}

//thread local storage for the memory manager
thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> = RefCell::new(
        MemoryManager::init(DefaultMemoryImpl::default())
    );

    static ID_COUNTER: RefCell<IdCell> = RefCell::new(
        IdCell::init(MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(0))), 0)
            .expect("Cannot create a counter")
    );

    static TIMBER_STORAGE: RefCell<StableBTreeMap<u64, Timber, Memory>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(1)))
    ));

    static SALES_STORAGE: RefCell<StableBTreeMap<u64, Sales, Memory>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(2)))
    ));
}

//a struct to hold the payload for the timber
#[derive(candid::CandidType, Serialize, Deserialize, Default)]
struct TimberPayload {
    timber_type: String,
    timber_size: String,
    quantity: u64,
}

//a struct to hold the payload for the sales
#[derive(candid::CandidType, Serialize, Deserialize, Default)]
struct SalesPayload {
    timber_id: u64,
    quantity: u64,
    price: u64,
}

//a struct to hold the payload for the sales
#[derive(candid::CandidType, Serialize, Deserialize, Default)]
struct SalesUpdatePayload {
    id: u64,
    quantity: u64,
    price: u64,
}

//a struct to hold the payload for the timber
#[derive(candid::CandidType, Serialize, Deserialize, Default)]
struct TimberUpdatePayload {
    id: u64,
    timber_type: String,
    timber_size: String,
    quantity: u64,
}

//function to get a timber by id
#[ic_cdk::query]
fn get_timber(id: u64) -> Result<Timber, String> {
    match _get_timber(&id) {
        Some(timber) => Ok(timber),
        None => Err(format!("a timber with id={} not found", id)),
    }
}


//function to get a sales by id
#[ic_cdk::query]
fn get_sales(id: u64) -> Result<Sales, String> {
    match _get_sales(&id) {
        Some(sales) => Ok(sales),
        None => Err(format!("a sales with id={} not found", id)),
    }
}

//function to add a timber
#[ic_cdk::update]
fn add_timber(timber: TimberPayload) -> Option<Timber> {
    let id = ID_COUNTER
        .with(|counter| {
            let current_value = *counter.borrow().get();
            counter.borrow_mut().set(current_value + 1)
        })
        .expect("cannot increment id counter");
    let timber = Timber {
        id,
        timber_type: timber.timber_type,
        timber_size: timber.timber_size,
        quantity: timber.quantity,
        created_at: time(),
        updated_at: None,
    };
    do_insert_timber(&timber);
    Some(timber)
}

//function to add a sales
#[ic_cdk::update]
fn add_sales(sales: SalesPayload) -> Option<Sales> {
    let id = ID_COUNTER
        .with(|counter| {
            let current_value = *counter.borrow().get();
            counter.borrow_mut().set(current_value + 1)
        })
        .expect("cannot increment id counter");
    let sales = Sales {
        id,
        timber_id: sales.timber_id,
        quantity: sales.quantity,
        price: sales.price,
        created_at: time(),
        updated_at: None,
    };
    do_insert_sales(&sales);
    Some(sales)
}

//function to update a timber
#[ic_cdk::update]
fn update_timber(id: u64, payload: TimberUpdatePayload) -> Result<Timber, String> {
    match TIMBER_STORAGE.with(|service| service.borrow().get(&id)) {
        Some(mut timber) => {
            timber.timber_type = payload.timber_type;
            timber.timber_size = payload.timber_size;
            timber.quantity = payload.quantity;
            timber.updated_at = Some(time());
            do_insert_timber(&timber);
            Ok(timber)
        }
        None => Err(format!(
            "couldn't update a timber with id={}. timber not found",
            id
        )),
    }
}

//function to update a sales
#[ic_cdk::update]
fn update_sales(id: u64, payload: SalesUpdatePayload) -> Result<Sales, String> {
    match SALES_STORAGE.with(|service| service.borrow().get(&id)) {
        Some(mut sales) => {
            sales.quantity = payload.quantity;
            sales.price = payload.price;
            sales.updated_at = Some(time());
            do_insert_sales(&sales);
            Ok(sales)
        }
        None => Err(format!(
            "couldn't update a sales with id={}. sales not found",
            id
        )),
    }
}

//function to delete a timber
#[ic_cdk::update]
fn delete_timber(id: u64) -> Result<Timber, String> {
    match TIMBER_STORAGE.with(|service| service.borrow().get(&id)) {
        Some(timber) => {
            TIMBER_STORAGE.with(|service| service.borrow_mut().remove(&id));
            Ok(timber)
        }
        None => Err(format!(
            "couldn't delete a timber with id={}. timber not found",
            id
        )),
    }
}

//function to delete a sales
#[ic_cdk::update]
fn delete_sales(id: u64) -> Result<Sales, String> {
    match SALES_STORAGE.with(|service| service.borrow().get(&id)) {
        Some(sales) => {
            SALES_STORAGE.with(|service| service.borrow_mut().remove(&id));
            Ok(sales)
        }
        None => Err(format!(
            "couldn't delete a sales with id={}. sales not found",
            id
        )),
    }
}


//helper method to perform insert.
fn do_insert_timber(timber: &Timber) {
    TIMBER_STORAGE.with(|service| service.borrow_mut().insert(timber.id, timber.clone()));
}

//helper method to perform insert.
fn do_insert_sales(sales: &Sales) {
    SALES_STORAGE.with(|service| service.borrow_mut().insert(sales.id, sales.clone()));
}

//helper method to get a timber by id. used in get_timber/update_timber
fn _get_timber(id: &u64) -> Option<Timber> {
    TIMBER_STORAGE.with(|service| service.borrow().get(id))
}

//helper method to get a sales by id. used in get_sales/update_sales
fn _get_sales(id: &u64) -> Option<Sales> {
    SALES_STORAGE.with(|service| service.borrow().get(id))
}

//helper method to get a sales by id. used in get_sales/update_sales
fn _get_timber_by_type(timber_type: &str) -> Vec<Timber> {
    TIMBER_STORAGE
        .with(|service| {
            service
                .borrow()
                .iter()
                .filter(|(_, timber)| timber.timber_type == timber_type)
                .map(|(_, timber)| timber.clone())
                .collect()
        })
}

//helper method to get a sales by id. used in get_sales/update_sales
fn _get_timber_by_size(timber_size: &str) -> Vec<Timber> {
    TIMBER_STORAGE
        .with(|service| {
            service
                .borrow()
                .iter()
                .filter(|(_, timber)| timber.timber_size == timber_size)
                .map(|(_, timber)| timber.clone())
                .collect()
        })
}


//helper method to get a sales by id. used in get_sales/update_sales
fn _get_sales_by_timber_id(timber_id: &u64) -> Vec<Sales> {
    SALES_STORAGE
        .with(|service| {
            service
                .borrow()
                .iter()
                .filter(|(_, sales)| sales.timber_id == *timber_id)
                .map(|(_, sales)| sales.clone())
                .collect()
        })
}

//helper method to get a sales by id. used in get_sales/update_sales
fn _get_sales_by_price(price: &u64) -> Vec<Sales> {
    SALES_STORAGE
        .with(|service| {
            service
                .borrow()
                .iter()
                .filter(|(_, sales)| sales.price == *price)
                .map(|(_, sales)| sales.clone())
                .collect()
        })
}

//helper method to get a sales by id. used in get_sales/update_sales
fn _get_sales_by_quantity(quantity: &u64) -> Vec<Sales> {
    SALES_STORAGE
        .with(|service| {
            service
                .borrow()
                .iter()
                .filter(|(_, sales)| sales.quantity == *quantity)
                .map(|(_, sales)| sales.clone())
                .collect()
        })
}

//helper method to get a sales by id. used in get_sales/update_sales
fn _get_sales_by_id(id: &u64) -> Option<Sales> {
    SALES_STORAGE.with(|service| service.borrow().get(id))
}

//helper method to get a sales by id. used in get_sales/update_sales
fn _get_timber_by_id(id: &u64) -> Option<Timber> {
    TIMBER_STORAGE.with(|service| service.borrow().get(id))
}

//helper method to get a sales by id. used in get_sales/update_sales
fn _get_timber_by_quantity(quantity: &u64) -> Vec<Timber> {
    TIMBER_STORAGE
        .with(|service| {
            service
                .borrow()
                .iter()
                .filter(|(_, timber)| timber.quantity == *quantity)
                .map(|(_, timber)| timber.clone())
                .collect()
        })
}


//helper method to get a sales by id. used in get_sales/update_sales
fn _get_timber_by_created_at(created_at: &u64) -> Vec<Timber> {
    TIMBER_STORAGE
        .with(|service| {
            service
                .borrow()
                .iter()
                .filter(|(_, timber)| timber.created_at == *created_at)
                .map(|(_, timber)| timber.clone())
                .collect()
        })
}

//helper method to get a sales by id. used in get_sales/update_sales
fn _get_sales_by_created_at(created_at: &u64) -> Vec<Sales> {
    SALES_STORAGE
        .with(|service| {
            service
                .borrow()
                .iter()
                .filter(|(_, sales)| sales.created_at == *created_at)
                .map(|(_, sales)| sales.clone())
                .collect()
        })
}

//helper method to get a sales by id. used in get_sales/update_sales
fn _get_timber_by_updated_at(updated_at: &u64) -> Vec<Timber> {
    TIMBER_STORAGE
        .with(|service| {
            service
                .borrow()
                .iter()
                .filter(|(_, timber)| timber.updated_at == Some(*updated_at))
                .map(|(_, timber)| timber.clone())
                .collect()
        })
}

//helper method to get a sales by id. used in get_sales/update_sales
fn _get_sales_by_updated_at(updated_at: &u64) -> Vec<Sales> {
    SALES_STORAGE
        .with(|service| {
            service
                .borrow()
                .iter()
                .filter(|(_, sales)| sales.updated_at == Some(*updated_at))
                .map(|(_, sales)| sales.clone())
                .collect()
        })
}

//helper method to get a sales by id. used in get_sales/update_sales
fn _get_timber_by_type_and_size(timber_type: &str, timber_size: &str) -> Vec<Timber> {
    TIMBER_STORAGE
        .with(|service| {
            service
                .borrow()
                .iter()
                .filter(|(_, timber)| {
                    timber.timber_type == timber_type && timber.timber_size == timber_size
                })
                .map(|(_, timber)| timber.clone())
                .collect()
        })
}

//helper method to get a sales by id. used in get_sales/update_sales
fn _get_timber_by_type_and_quantity(timber_type: &str, quantity: &u64) -> Vec<Timber> {
    TIMBER_STORAGE
        .with(|service| {
            service
                .borrow()
                .iter()
                .filter(|(_, timber)| timber.timber_type == timber_type && timber.quantity == *quantity)
                .map(|(_, timber)| timber.clone())
                .collect()
        })
}

//helper method to get a sales by id. used in get_sales/update_sales
fn _get_timber_by_size_and_quantity(timber_size: &str, quantity: &u64) -> Vec<Timber> {
    TIMBER_STORAGE
        .with(|service| {
            service
                .borrow()
                .iter()
                .filter(|(_, timber)| timber.timber_size == timber_size && timber.quantity == *quantity)
                .map(|(_, timber)| timber.clone())
                .collect()
        })
}


//helper method to get a sales by id. used in get_sales/update_sales
fn _get_timber_by_type_and_size_and_quantity(timber_type: &str, timber_size: &str, quantity: &u64) -> Vec<Timber> {
    TIMBER_STORAGE
        .with(|service| {
            service
                .borrow()
                .iter()
                .filter(|(_, timber)| {
                    timber.timber_type == timber_type && timber.timber_size == timber_size && timber.quantity == *quantity
                })
                .map(|(_, timber)| timber.clone())
                .collect()
        })
}



//need this to generate candid
ic_cdk::export_candid!();
