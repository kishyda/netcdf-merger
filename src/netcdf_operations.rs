use crate::helpers;
use netcdf::types::{FloatType, IntType, NcVariableType};
use netcdf::{File, Variable};

pub fn combine_netcdf_bytes(parts: &[&[u8]]) -> helpers::ApiResult<Vec<u8>> {
    let mut output = netcdf::create_mem(1024 * 1024)?;

    for part in parts {
        let input = netcdf::open_mem(None, part)?;
        merge_dimensions(&input, &mut output)?;
        merge_global_attributes(&input, &mut output)?;
        define_variables(&input, &mut output)?;
    }

    output.enddef()?;

    for part in parts {
        let input = netcdf::open_mem(None, part)?;
        copy_variable_data(&input, &mut output)?;
    }

    Ok(output.close_to_bytes()?)
}

fn merge_dimensions(input: &File, output: &mut netcdf::FileMut) -> helpers::ApiResult<()> {
    for dimension in input.dimensions() {
        let name = dimension.name();
        if output.dimension(&name).is_some() {
            continue;
        }

        if dimension.is_unlimited() {
            output.add_unlimited_dimension(&name)?;
        } else {
            output.add_dimension(&name, dimension.len())?;
        }
    }

    Ok(())
}

fn merge_global_attributes(input: &File, output: &mut netcdf::FileMut) -> helpers::ApiResult<()> {
    for attribute in input.attributes() {
        if output.attribute(attribute.name()).is_some() {
            continue;
        }

        let value = attribute.value()?;
        output.add_attribute(attribute.name(), value)?;
    }

    Ok(())
}

fn define_variables(input: &File, output: &mut netcdf::FileMut) -> helpers::ApiResult<()> {
    for variable in input.variables() {
        let name = variable.name();
        if output.variable(&name).is_some() {
            continue;
        }

        let dim_names = variable
            .dimensions()
            .iter()
            .map(|dimension| dimension.name())
            .collect::<Vec<_>>();
        let dim_refs = dim_names.iter().map(String::as_str).collect::<Vec<_>>();
        let variable_type = variable.vartype();

        let mut output_variable =
            output.add_variable_with_type(&name, &dim_refs, &variable_type)?;

        for attribute in variable.attributes() {
            if output_variable.attribute(attribute.name()).is_some() {
                continue;
            }

            let value = attribute.value()?;
            output_variable.put_attribute(attribute.name(), value)?;
        }
    }

    Ok(())
}

fn copy_variable_data(input: &File, output: &mut netcdf::FileMut) -> helpers::ApiResult<()> {
    for variable in input.variables() {
        let name = variable.name();
        let Some(mut output_variable) = output.variable_mut(&name) else {
            continue;
        };

        let variable_type = variable.vartype();
        copy_variable_values(&variable, &mut output_variable, &variable_type)?;
    }

    Ok(())
}

fn copy_variable_values(
    input: &Variable<'_>,
    output: &mut netcdf::VariableMut<'_>,
    variable_type: &NcVariableType,
) -> helpers::ApiResult<()> {
    match variable_type {
        NcVariableType::Int(IntType::U8) => copy_typed_values::<u8>(input, output)?,
        NcVariableType::Int(IntType::U16) => copy_typed_values::<u16>(input, output)?,
        NcVariableType::Int(IntType::U32) => copy_typed_values::<u32>(input, output)?,
        NcVariableType::Int(IntType::U64) => copy_typed_values::<u64>(input, output)?,
        NcVariableType::Int(IntType::I8) => copy_typed_values::<i8>(input, output)?,
        NcVariableType::Int(IntType::I16) => copy_typed_values::<i16>(input, output)?,
        NcVariableType::Int(IntType::I32) => copy_typed_values::<i32>(input, output)?,
        NcVariableType::Int(IntType::I64) => copy_typed_values::<i64>(input, output)?,
        NcVariableType::Float(FloatType::F32) => copy_typed_values::<f32>(input, output)?,
        NcVariableType::Float(FloatType::F64) => copy_typed_values::<f64>(input, output)?,
        unsupported => {
            return Err(helpers::ApiError::internal(format!(
                "unsupported variable type for merge: {unsupported:?}"
            )));
        }
    }

    Ok(())
}

fn copy_typed_values<T>(
    input: &Variable<'_>,
    output: &mut netcdf::VariableMut<'_>,
) -> helpers::ApiResult<()>
where
    T: netcdf::NcTypeDescriptor + Copy,
{
    let values = input.get_values::<T, _>(..)?;
    output.put_values(&values, ..)?;
    Ok(())
}
