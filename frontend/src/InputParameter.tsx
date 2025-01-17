import React from "react";

interface InputParameter {
    name: string;
    type: string;
    value: string;
}

const InputParameter = (props: InputParameter) => {
    return (
        <tr>
            <td>{props.name}</td>
            <td>{props.type}</td>
            <td>
                <input type="text" value={props.value}></input>
            </td>
        </tr>
    );
};
export default InputParameter;
