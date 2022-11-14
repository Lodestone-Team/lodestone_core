// a wrapper around TextField that fetches a single setting from the server
import { InstanceInfo } from 'bindings/InstanceInfo';
import { useGameSetting } from 'data/GameSetting';
import { axiosPutSingleValue, errorToMessage } from 'utils/util';
import Dropdown from './Atoms/Config/SelectBox';
import Textfield from './Atoms/Config/InputBox';
import { useState } from 'react';
import { useIsomorphicLayoutEffect } from 'usehooks-ts';
import { useUserAuthorized } from 'data/UserInfo';

export default function SettingField({
  instance,
  setting,
  label,
  type = 'text',
  options,
  min,
  max,
}: {
  instance: InstanceInfo;
  setting: string;
  label?: string;
  type?: 'text' | 'number' | 'dropdown';
  min?: number;
  max?: number;
  options?: string[];
}) {
  const uuid = instance.uuid;
  const {
    data: initialSetting,
    isLoading,
    error,
  } = useGameSetting(uuid, setting);
  label = label ?? setting;
  const [value, setValue] = useState(initialSetting ?? '');
  const can_access_instance_setting = useUserAuthorized(
    'can_access_instance_setting',
    instance.uuid
  );

  useIsomorphicLayoutEffect(() => {
    setValue(initialSetting ?? '');
  }, [initialSetting]);

  const errorString = errorToMessage(error);

  switch (type) {
    case 'text':
      return (
        <Textfield
          label={label}
          value={value}
          type="text"
          disabled={isLoading}
          error={errorString}
          onSubmit={async (value) => {
            await axiosPutSingleValue<void>(
              `/instance/${uuid}/game/${setting}`,
              value
            );
            setValue(value);
          }}
        />
      );
    case 'number':
      const numValue = parseInt(value);

      return (
        <Textfield
          label={label}
          value={value}
          type="number"
          min={min}
          max={max}
          disabled={isLoading}
          error={errorString}
          onSubmit={async (value) => {
            await axiosPutSingleValue<void>(
              `/instance/${uuid}/game/${setting}`,
              value
            );
          }}
        />
      );
    case 'dropdown':
      if (!options) {
        throw new Error('Dropdown type requires options');
      }
      return (
        <Dropdown
          label={label}
          value={value}
          options={options}
          disabled={isLoading}
          error={errorString}
          onChange={async (value) => {
            await axiosPutSingleValue<void>(
              `/instance/${uuid}/game/${setting}`,
              value
            );
          }}
        />
      );
  }
}